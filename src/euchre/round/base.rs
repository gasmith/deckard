//! Core round implementation.

use std::collections::{HashMap, VecDeque};

use super::{
    Action, ActionData, ActionType, Card, Contract, Event, ExpectAction, PlayerError, PlayerState,
    Round, RoundConfig, RoundError, Seat, Suit, Trick, Tricks,
};

/// The core implementation for [`Round`], around which other implementations are built.
#[derive(Debug)]
pub struct BaseRound {
    /// The dealer for this round.
    dealer: Seat,
    /// The upturned card.
    top: Card,
    /// The content of each player's hand.
    hands: HashMap<Seat, Vec<Card>>,
    /// The established contract, once bidding is over.
    contract: Option<Contract>,
    /// Tricks played during this round.
    tricks: Tricks,
    /// A queue of unacknowledged events.
    events: VecDeque<Event>,
    /// The next action required to advance the round.
    next_action: Option<ExpectAction>,
}

impl From<RoundConfig> for BaseRound {
    fn from(config: RoundConfig) -> Self {
        let dealer = config.dealer;
        let top = config.top;
        BaseRound {
            dealer,
            top,
            hands: config.hands,
            contract: None,
            tricks: Tricks::default(),
            events: [Event::Deal(dealer, top)].into(),
            next_action: Some(ExpectAction::new(dealer.next(), ActionType::BidTop)),
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
            (None, _) => Err(RoundError::RoundOver),
            (Some(ExpectAction { seat, action }), a) if seat != a.seat || action != a.action => {
                Err(RoundError::ExpectActioned { seat, action })
            }
            (_, a) => self.apply(a),
        }
    }
}

/// Filters the teammate for a loner hand.
fn filter_seat(contract: Contract, seat: Seat) -> Seat {
    if contract.alone && seat == contract.maker.opposite() {
        seat.next()
    } else {
        seat
    }
}

impl BaseRound {
    /// Applies the specified action to advance the state machine.
    fn apply(&mut self, Action { seat, action, data }: Action) -> Result<(), RoundError> {
        match (action, data) {
            (ActionType::BidTop, ActionData::Pass) => self.pass_top(seat),
            (ActionType::BidTop, ActionData::Call { suit, alone }) => {
                self.bid_top(seat, suit, alone)?;
            }
            (ActionType::BidOther, ActionData::Pass) => self.pass_other(seat)?,
            (ActionType::BidOther, ActionData::Call { suit, alone }) => {
                self.bid_other(seat, suit, alone)?;
            }
            (ActionType::DealerDiscard, ActionData::Card { card }) => {
                self.dealer_discard(seat, card)?;
            }
            (ActionType::Lead, ActionData::Card { card }) => self.lead(seat, card)?,
            (ActionType::Follow, ActionData::Card { card }) => self.follow(seat, card)?,
            _ => return Err(RoundError::InvalidActionData),
        }
        Ok(())
    }

    /// Handles the case where the player declines to order up the top card.
    fn pass_top(&mut self, seat: Seat) {
        if seat == self.dealer {
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
        } else {
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidTop));
        }
    }

    /// Handles the case where the player order up the top card.
    fn bid_top(&mut self, maker: Seat, suit: Suit, alone: bool) -> Result<(), PlayerError> {
        if suit == self.top.suit {
            let contract = Contract { maker, suit, alone };
            self.contract = Some(contract);
            self.hands
                .get_mut(&self.dealer)
                .expect("hands populated")
                .push(self.top);
            // If some player other than the dealer bids top alone, the top card is simply buried
            // with the rest of the dealer's hand - no need to discard.
            if alone && maker != self.dealer {
                self.first_trick();
            } else {
                self.next_action = Some(ExpectAction::new(self.dealer, ActionType::DealerDiscard));
            }
            self.events.push_back(Event::Call(contract));
            Ok(())
        } else {
            Err(PlayerError::MustCallTopSuit(self.top.suit))
        }
    }

    /// Handles the case where the player declines to call an alternative suit.
    fn pass_other(&mut self, seat: Seat) -> Result<(), PlayerError> {
        if seat == self.dealer {
            Err(PlayerError::DealerMustBidOther)
        } else {
            self.next_action = Some(ExpectAction::new(seat.next(), ActionType::BidOther));
            Ok(())
        }
    }

    /// Handles the case where the player calls an alternative suit.
    fn bid_other(&mut self, maker: Seat, suit: Suit, alone: bool) -> Result<(), PlayerError> {
        if suit == self.top.suit {
            Err(PlayerError::CannotCallTopSuit(self.top.suit))
        } else {
            let contract = Contract { maker, suit, alone };
            self.contract = Some(contract);
            self.first_trick();
            self.events.push_back(Event::Call(contract));
            Ok(())
        }
    }

    /// Handles the case where the dealer discards a card after picking up the top card.
    fn dealer_discard(&mut self, dealer: Seat, card: Card) -> Result<(), PlayerError> {
        assert_eq!(dealer, self.dealer);
        self.find_and_discard(dealer, card)?;
        self.first_trick();
        Ok(())
    }

    /// Handles the start of a new trick.
    fn lead(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        let contract = self.contract.expect("contract must be set");
        self.find_and_discard(seat, card)?;
        let trick = Trick::new(contract.suit, seat, card);
        self.tricks.push(trick);
        self.next_action = Some(ExpectAction::new(
            filter_seat(contract, seat.next()),
            ActionType::Follow,
        ));
        Ok(())
    }

    /// Handles the play of a card into a pending trick.
    fn follow(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        let contract = self.contract.expect("contract must be set");
        let index = self.find_card(seat, card)?;

        let trick_size = self.tricks.trick_size();
        let trick = self.tricks.last_mut().expect("trick must be started");
        assert!(trick.len() < trick_size);

        let hand = self.hands.get_mut(&seat).expect("hand exists");
        if !trick.is_following_lead(hand, card) {
            return Err(PlayerError::MustFollowLead(seat, trick.lead().1));
        }

        trick.play(seat, card);
        hand.remove(index);

        if trick.len() < trick_size {
            self.next_action = Some(ExpectAction::new(
                filter_seat(contract, seat.next()),
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

    /// Finds a card among the specified player's hand and discards it.
    fn find_and_discard(&mut self, seat: Seat, card: Card) -> Result<(), PlayerError> {
        self.find_card(seat, card)
            .map(|index| self.discard(seat, index))
    }

    /// Finds a card among the specified player's hand.
    fn find_card(&mut self, seat: Seat, card: Card) -> Result<usize, PlayerError> {
        self.hands
            .get(&seat)
            .expect("hand exists")
            .iter()
            .position(|c| *c == card)
            .ok_or(PlayerError::CardNotHeld(seat, card))
    }

    /// Discards the specified card from the player's hand.
    fn discard(&mut self, seat: Seat, index: usize) {
        let hand = self.hands.get_mut(&seat).expect("hand exists");
        hand.remove(index);
    }

    /// Sets up the state machine for the first trick, choosing the eldest hand to lead.
    fn first_trick(&mut self) {
        let contract = self
            .contract
            .as_ref()
            .copied()
            .expect("contract must be set");
        if contract.alone {
            self.tricks.set_trick_size(3);
        }
        let seat = filter_seat(contract, self.dealer.next());
        self.next_trick(seat);
    }

    /// Sets up the state machine for the next trick.
    fn next_trick(&mut self, seat: Seat) {
        self.next_action = Some(ExpectAction {
            seat,
            action: ActionType::Lead,
        });
    }
}
