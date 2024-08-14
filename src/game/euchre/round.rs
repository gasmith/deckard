//! Round management

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;

use delegate::delegate;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{
    Action, ActionData, ActionType, Card, Contract, Deck, Event, ExpectAction, PlayerError,
    RoundError, Seat, Suit, Team, Trick,
};

mod log;
pub use log::{Id, Log, RawLog};

/// The outcome of a round.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub team: Team,
    pub points: u8,
}
impl Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} wins {} points", self.team, self.points)
    }
}

impl Outcome {
    pub fn new(team: Team, points: u8) -> Self {
        Outcome { team, points }
    }
}

/// Initial conditions for a round.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InitialState {
    dealer: Seat,
    hands: HashMap<Seat, Vec<Card>>,
    top: Card,
}
impl Distribution<InitialState> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> InitialState {
        InitialState::new(rng.gen(), rng.gen()).expect("deck is valid")
    }
}

/// Initial state for a [`Round`]
impl InitialState {
    fn new(dealer: Seat, mut deck: Deck) -> Result<Self, RoundError> {
        if deck.len() < 24 {
            return Err(RoundError::IncompleteDeck);
        }
        let hands = dealer
            .next_n(4)
            .into_iter()
            .map(|seat| (seat, deck.take(5)))
            .collect();
        let top = deck.take(1)[0];
        Self { dealer, hands, top }.validate()
    }

    fn validate(self) -> Result<Self, RoundError> {
        let mut seen: HashSet<_> = self
            .hands
            .values()
            .flat_map(|cards| cards.iter().copied())
            .collect();
        seen.insert(self.top);
        if seen.len() == 21 {
            Ok(self)
        } else {
            Err(RoundError::DuplicateCard)
        }
    }
}

/// The main state machine for the round.
///
/// A new round is initiated by a deal into the `BidTop` state. The seats
/// then bid for the top card. If a bid is made, the top card is replaces a
/// card in the dealer's hand, and the round progresses to `Play`.
///
/// Otherwise, the state machine advances to `BidOther`, where seats bid for
/// any suit other than that of the top card. The dealer is required to choose
/// a suit, if no other seat will.
#[derive(Debug)]
pub struct Round {
    dealer: Seat,
    hands: HashMap<Seat, Vec<Card>>,
    top: Card,
    contract: Option<Contract>,
    tricks: Vec<Trick>,
    events: VecDeque<Event>,
    next: Option<ExpectAction>,
}
impl From<InitialState> for Round {
    fn from(initial: InitialState) -> Self {
        Round {
            dealer: initial.dealer,
            hands: initial.hands,
            top: initial.top,
            contract: None,
            tricks: vec![],
            events: [Event::Deal(initial.dealer, initial.top)].into(),
            next: Some(ExpectAction::new(initial.dealer.next(), ActionType::BidTop)),
        }
    }
}

/// Filters the teammate for a loner hand.
fn filter_seat(contract: &Contract, seat: Seat) -> Seat {
    if contract.alone && seat == contract.maker.opposite() {
        seat.next()
    } else {
        seat
    }
}

impl Round {
    pub fn random() -> Self {
        rand::random::<InitialState>().into()
    }

    pub fn new(dealer: Seat, deck: Deck) -> Result<Self, RoundError> {
        InitialState::new(dealer, deck).map(Self::from)
    }

    pub fn next(&self) -> Option<ExpectAction> {
        self.next
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    pub fn player_state(&self, seat: Seat) -> PlayerState<'_> {
        PlayerState {
            seat,
            dealer: self.dealer,
            top: self.top,
            contract: self.contract,
            hand: self.hands.get(&seat).expect("seats populated"),
            tricks: &self.tricks,
        }
    }

    pub fn apply(&mut self, action: Action) -> Result<(), RoundError> {
        match (self.next, action) {
            (None, _) => Err(RoundError::GameOver),
            (Some(ExpectAction { seat, action }), a) if seat != a.seat || action != a.action => {
                Err(RoundError::ExpectActioned { seat, action })
            }
            (_, a) => self.handle(a),
        }
    }

    fn handle(&mut self, Action { seat, action, data }: Action) -> Result<(), RoundError> {
        match (action, data) {
            (ActionType::BidTop, ActionData::Pass) => self.pass_top(seat),
            (ActionType::BidTop, ActionData::BidTop { alone }) => self.bid_top(seat, alone),
            (ActionType::BidOther, ActionData::Pass) => self.pass_other(seat)?,
            (ActionType::BidOther, ActionData::BidOther { suit, alone }) => {
                self.bid_other(seat, suit, alone)?
            }
            (ActionType::DealerDiscard, ActionData::Card { card }) => {
                self.dealer_discard(seat, card)?
            }
            (ActionType::Lead, ActionData::Card { card }) => self.lead(seat, card)?,
            (ActionType::Follow, ActionData::Card { card }) => self.follow(seat, card)?,
            _ => return Err(RoundError::InvalidActionData),
        }
        Ok(())
    }

    fn pass_top(&mut self, seat: Seat) {
        if seat == self.dealer {
            self.next = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
        } else {
            self.next = Some(ExpectAction::new(seat.next(), ActionType::BidTop));
        }
    }

    fn bid_top(&mut self, maker: Seat, alone: bool) {
        let contract = Contract {
            maker,
            suit: self.top.suit,
            alone,
        };
        self.contract = Some(contract);
        self.hands
            .get_mut(&self.dealer)
            .expect("hands populated")
            .push(self.top);
        self.next = Some(ExpectAction::new(self.dealer, ActionType::DealerDiscard));
        self.events.push_back(Event::Bid(contract));
    }

    fn pass_other(&mut self, seat: Seat) -> Result<(), PlayerError> {
        if seat == self.dealer {
            Err(PlayerError::DealerMustBidOther)
        } else {
            self.next = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
            Ok(())
        }
    }

    fn bid_other(&mut self, maker: Seat, suit: Suit, alone: bool) -> Result<(), PlayerError> {
        if suit == self.top.suit {
            Err(PlayerError::CannotBidTopSuit(self.top.suit))
        } else {
            let contract = Contract { maker, suit, alone };
            self.contract = Some(contract);
            self.first_trick();
            self.events.push_back(Event::Bid(contract));
            Ok(())
        }
    }

    fn dealer_discard(&mut self, dealer: Seat, card: Card) -> Result<(), PlayerError> {
        assert_eq!(dealer, self.dealer);
        self.find_and_discard(dealer, card)?;
        self.first_trick();
        Ok(())
    }

    fn lead(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        let contract = self.contract.expect("contract must be set");
        self.find_and_discard(seat, card)?;
        let trick = Trick::new(contract.suit, seat, card);
        self.tricks.push(trick);
        self.next = Some(ExpectAction::new(
            filter_seat(&contract, seat.next()),
            ActionType::Follow,
        ));
        Ok(())
    }

    fn follow(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        let contract = self.contract.expect("contract must be set");
        let index = self.find_card(seat, card)?;

        assert!(self.tricks.len() <= 5);
        let trick = self.tricks.last_mut().expect("trick must be started");
        let trick_size = if contract.alone { 3 } else { 4 };
        assert!(trick.len() < trick_size);

        let hand = self.hands.get_mut(&seat).expect("hand exists");
        if !trick.is_following_lead(hand, &card) {
            return Err(PlayerError::MustFollowLead(seat, trick.lead().1));
        }

        trick.play(seat, card);
        hand.remove(index);

        if trick.len() < trick_size {
            self.next = Some(ExpectAction::new(
                filter_seat(&contract, seat.next()),
                ActionType::Follow,
            ));
        } else {
            let winner = trick.best().0;
            self.events.push_back(Event::Trick(trick.clone()));
            if let Some(outcome) = self.outcome() {
                self.events.push_back(Event::Round(outcome));
                self.next = None;
            } else {
                self.next_trick(winner);
            }
        }

        Ok(())
    }

    fn find_and_discard(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        self.find_card(seat, card)
            .map(|index| self.discard(seat, index))
    }

    fn find_card(&mut self, seat: Seat, card: Card) -> Result<usize, PlayerError> {
        self.hands
            .get(&seat)
            .expect("hand exists")
            .iter()
            .position(|c| *c == card)
            .ok_or(PlayerError::CardNotHeld(seat, card))
    }

    fn discard(&mut self, seat: Seat, index: usize) {
        let hand = self.hands.get_mut(&seat).expect("hand exists");
        hand.remove(index);
    }

    fn first_trick(&mut self) {
        let contract = self.contract.as_ref().expect("contract must be set");
        let seat = filter_seat(contract, self.dealer.next());
        self.next_trick(seat);
    }

    fn next_trick(&mut self, seat: Seat) {
        self.next = Some(ExpectAction {
            seat,
            action: ActionType::Lead,
        });
    }

    pub fn trick_counts(&self) -> HashMap<Team, u8> {
        let mut count: HashMap<_, u8> = HashMap::new();
        for t in &self.tricks {
            *count.entry(Team::from(t.best().0)).or_default() += 1;
        }
        count
    }

    pub fn outcome(&self) -> Option<Outcome> {
        let contract = self.contract.expect("contract must be set");
        let counts = self.trick_counts();
        let makers = Team::from(contract.maker);
        let makers_tricks = counts.get(&makers).copied().unwrap_or(0).into();
        let total_tricks = self.tricks.len();
        if total_tricks - makers_tricks >= 3 {
            // Euchred! No need to keep playing.
            let defenders = makers.other();
            Some(Outcome::new(defenders, 2))
        } else if total_tricks == 5 {
            // All tricks have been played, and the makers were not euchred.
            match (makers_tricks, contract.alone) {
                (5, true) => Some(Outcome::new(makers, 4)),
                (5, false) => Some(Outcome::new(makers, 2)),
                _ => Some(Outcome::new(makers, 1)),
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PlayerState<'a> {
    pub seat: Seat,
    pub dealer: Seat,
    pub top: Card,
    pub contract: Option<Contract>,
    pub hand: &'a Vec<Card>,
    pub tricks: &'a Vec<Trick>,
}

impl<'a> PlayerState<'a> {
    pub fn current_trick(&self) -> Option<&Trick> {
        self.tricks.last()
    }

    pub fn trick_counts(&self) -> HashMap<Team, u8> {
        let mut count: HashMap<_, u8> = HashMap::new();
        if let Some(contract) = self.contract {
            let trick_size = if contract.alone { 3 } else { 4 };
            for t in self.tricks {
                if t.len() == trick_size {
                    *count.entry(Team::from(t.best().0)).or_default() += 1;
                }
            }
        }
        count
    }
}

#[derive(Debug)]
pub struct LoggingRound {
    round: Round,
    log: Log,
    cursor: Option<Id>,
}
impl From<InitialState> for LoggingRound {
    fn from(initial: InitialState) -> Self {
        Self {
            log: Log::new(initial.clone()),
            round: initial.into(),
            cursor: None,
        }
    }
}
impl From<LoggingRound> for RawLog {
    fn from(value: LoggingRound) -> Self {
        value.log.into()
    }
}

impl LoggingRound {
    pub fn random() -> Self {
        rand::random::<InitialState>().into()
    }

    delegate! {
        to self.round {
            pub fn next(&self) -> Option<ExpectAction>;
            pub fn pop_event(&mut self) -> Option<Event>;
            pub fn player_state(&self, seat: Seat) -> PlayerState<'_>;
            pub fn trick_counts(&self) -> HashMap<Team, u8>;
            pub fn outcome(&self) -> Option<Outcome>;
        }
    }

    pub fn apply(&mut self, action: Action) -> Result<(), RoundError> {
        self.round.apply(action)?;
        self.cursor = Some(self.log.insert(self.cursor, action));
        Ok(())
    }

    fn seek(&mut self, id: Id) -> Result<(), RoundError> {
        self.restart();
        for (id, action) in self.log.backtrace(id)? {
            self.round.apply(action).expect("re-apply always works");
            self.cursor = Some(id);
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), RoundError> {
        if let Some(id) = self.cursor {
            self.seek(id)
        } else {
            self.restart();
            Ok(())
        }
    }

    pub fn restart(&mut self) {
        self.cursor = None;
        self.round = Round::from(self.log.initial().clone());
    }
}
