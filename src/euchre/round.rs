//! BaseRound management

use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{
    Action, ActionData, ActionType, Card, Contract, Deck, Event, ExpectAction, PlayerError,
    RoundError, Seat, Suit, Team, Trick,
};

mod base;
mod log;
mod logging;
mod tricks;
pub use base::BaseRound;
pub use log::{Id as LogId, Log, RawLog};
pub use logging::LoggingRound;
pub use tricks::Tricks;

pub trait Round {
    /// The dealer of this round.
    fn dealer(&self) -> Seat;

    /// The top card from this round.
    fn top_card(&self) -> Card;

    /// Returns the next action that's required to advance the state of the
    /// round, or None if the round is over.
    fn next_action(&self) -> Option<ExpectAction>;

    /// The declared contract.
    fn contract(&self) -> Option<Contract>;

    /// Tricks played during this round.
    fn tricks(&self) -> &Tricks;

    /// Returns a bundle of state visible to the specified player.
    fn player_state(&self, seat: Seat) -> PlayerState<'_>;

    /// Applies the specified action.
    fn apply_action(&mut self, action: Action) -> Result<(), RoundError>;

    /// Pops the oldest event from the queue of events.
    fn pop_event(&mut self) -> Option<Event>;

    /// The outcome of the round, if it is over.
    fn outcome(&self) -> Option<RoundOutcome> {
        let Some(contract) = self.contract() else {
            return None;
        };
        let makers = Team::from(contract.maker);
        let defenders = makers.other();

        let tricks = self.tricks();
        let makers_count = tricks.win_count(makers);
        let defenders_count = tricks.win_count(defenders);

        if defenders_count >= 3 {
            // Euchred! No need to keep playing.
            let defenders = makers.other();
            Some(RoundOutcome::new(defenders, 2))
        } else if makers_count + defenders_count == 5 {
            // All tricks have been played, and the makers were not euchred.
            match (makers_count, contract.alone) {
                (5, true) => Some(RoundOutcome::new(makers, 4)),
                (5, false) => Some(RoundOutcome::new(makers, 2)),
                _ => Some(RoundOutcome::new(makers, 1)),
            }
        } else {
            None
        }
    }
}

/// Configuration & initial conditions for a round.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundConfig {
    dealer: Seat,
    hands: HashMap<Seat, Vec<Card>>,
    top: Card,
}

impl Distribution<RoundConfig> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RoundConfig {
        RoundConfig::new(rng.gen(), rng.gen()).expect("deck is valid")
    }
}

impl RoundConfig {
    pub fn new(dealer: Seat, mut deck: Deck) -> Result<Self, RoundError> {
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

    pub fn random() -> Self {
        rand::random::<RoundConfig>().into()
    }

    pub fn random_with_dealer(dealer: Seat) -> Self {
        let deck = rand::random();
        Self::new(dealer, deck).expect("deck is valid")
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

/// The outcome of a round.
#[derive(Debug, Clone)]
pub struct RoundOutcome {
    pub team: Team,
    pub points: u8,
}

impl Display for RoundOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} wins {} points", self.team, self.points)
    }
}

impl RoundOutcome {
    pub fn new(team: Team, points: u8) -> Self {
        RoundOutcome { team, points }
    }
}

/// The state visible to a particular seat.
#[derive(Debug)]
pub struct PlayerState<'a> {
    pub seat: Seat,
    pub dealer: Seat,
    pub top: Card,
    pub contract: Option<Contract>,
    pub hand: &'a Vec<Card>,
    pub tricks: &'a Tricks,
}

impl<'a> PlayerState<'a> {
    pub fn new(
        seat: Seat,
        dealer: Seat,
        top: Card,
        contract: Option<Contract>,
        hand: &'a Vec<Card>,
        tricks: &'a Tricks,
    ) -> Self {
        Self {
            seat,
            dealer,
            top,
            contract,
            hand,
            tricks,
        }
    }

    // TODO: Maybe build a Hand abstraction?
    pub fn sorted_hand(&self) -> Vec<Card> {
        let mut cards = self.hand.clone();
        if let Some(contract) = self.contract {
            cards.sort_unstable_by_key(|c| {
                (c.effective_suit(contract.suit), c.value(contract.suit, *c))
            });
        } else {
            cards.sort_unstable_by_key(|c| (c.suit, c.rank));
        }
        cards
    }
}
