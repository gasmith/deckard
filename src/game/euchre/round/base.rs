//! Base round.

use std::collections::{HashMap, VecDeque};

use super::{
    Action, ActionData, ActionType, Card, Contract, Deck, Event, ExpectAction, InitialState,
    PlayerError, PlayerState, Round, RoundError, Seat, Suit, Trick, Tricks,
};

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
pub struct BaseRound {
    dealer: Seat,
    hands: HashMap<Seat, Vec<Card>>,
    top: Card,
    contract: Option<Contract>,
    tricks: Tricks,
    events: VecDeque<Event>,
    next_action: Option<ExpectAction>,
}

impl From<InitialState> for BaseRound {
    fn from(initial: InitialState) -> Self {
        BaseRound {
            dealer: initial.dealer,
            hands: initial.hands,
            top: initial.top,
            contract: None,
            tricks: Tricks::default(),
            events: [Event::Deal(initial.dealer, initial.top)].into(),
            next_action: Some(ExpectAction::new(initial.dealer.next(), ActionType::BidTop)),
        }
    }
}

impl Round for BaseRound {
    fn dealer(&self) -> Seat {
        self.dealer
    }

    fn top_card(&self) -> Card {
        self.top
    }

    fn pop_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    fn next_action(&self) -> Option<ExpectAction> {
        self.next_action
    }

    fn contract(&self) -> Option<Contract> {
        self.contract
    }

    fn tricks(&self) -> &Tricks {
        &self.tricks
    }

    fn player_state(&self, seat: Seat) -> PlayerState<'_> {
        PlayerState::new(
            seat,
            self.dealer,
            self.top,
            self.contract,
            self.hands.get(&seat).expect("seats populated"),
            &self.tricks,
        )
    }

    fn apply_action(&mut self, action: Action) -> Result<(), RoundError> {
        match (self.next_action, action) {
            (None, _) => Err(RoundError::GameOver),
            (Some(ExpectAction { seat, action }), a) if seat != a.seat || action != a.action => {
                Err(RoundError::ExpectActioned { seat, action })
            }
            (_, a) => self.handle(a),
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

impl BaseRound {
    pub fn random() -> Self {
        rand::random::<InitialState>().into()
    }

    pub fn new(dealer: Seat, deck: Deck) -> Result<Self, RoundError> {
        InitialState::new(dealer, deck).map(Self::from)
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
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
        } else {
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidTop));
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
        self.next_action = Some(ExpectAction::new(self.dealer, ActionType::DealerDiscard));
        self.events.push_back(Event::Bid(contract));
    }

    fn pass_other(&mut self, seat: Seat) -> Result<(), PlayerError> {
        if seat == self.dealer {
            Err(PlayerError::DealerMustBidOther)
        } else {
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
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
        self.next_action = Some(ExpectAction::new(
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
            self.next_action = Some(ExpectAction::new(
                filter_seat(&contract, seat.next()),
                ActionType::Follow,
            ));
        } else {
            let winner = trick.best().0;
            self.events.push_back(Event::Trick(trick.clone()));
            if let Some(outcome) = self.outcome() {
                self.events.push_back(Event::Round(outcome));
                self.next_action = None;
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
        if contract.alone {
            self.tricks.set_trick_size(3);
        }
        let seat = filter_seat(contract, self.dealer.next());
        self.next_trick(seat);
    }

    fn next_trick(&mut self, seat: Seat) {
        self.next_action = Some(ExpectAction {
            seat,
            action: ActionType::Lead,
        });
    }
}
