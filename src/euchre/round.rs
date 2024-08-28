//! Round management

use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{
    Action, ActionData, ActionType, Card, Deck, Event, ExpectAction, PlayerError, RoundError, Seat,
    Suit, Team, Trick,
};

mod base;
mod log;
mod logging;
mod tricks;
pub use base::BaseRound;
pub use log::{Id as LogId, Log, RawLog};
pub use logging::LoggingRound;
pub use tricks::Tricks;

/// A trait for implementing a round of euchre.
///
/// ## Gameplay
///
/// A round is initiated by a deal, and then players bid for trump in clockwise order. Each player
/// is given an opportunity to "order up" the top card into the dealer's hand and declare its suit
/// as trump. When a player does so, the dealer acquires the top card, discards another card from
/// their hand, and tricks begin.
///
/// If all players decline to make a contract over the top card, then each player is given an
/// opportunity to name an alternative suit as trump. The dealer is forced to choose, if no one
/// else will.
///
/// The first trick is led by the next player clockwise from the dealer. Each trick involves a card
/// played, in clockwise order, from each player. When the trick is complete, the player who played
/// the highest-valued card wins the trick and leads the next.
///
/// The round ends when all five tricks have been played, or when the defenders manage to
/// win their third trick (and thereby "euchres" the makers). The score of the round is calculated
/// based on the style of the contract, and the number of tricks taken.
///
/// ## State management
///
/// The round begins in an initial state, after cards have been dealt, and the top card has been
/// upturned. To advance the state of the round, players are required to take actions. The identity
/// of the next player and the action they are expected to take is always known deterministically,
/// and may be obtained via [`next_action`](`Round::next_action`)
///
/// Once a player has chosen an action, it is applied using
/// [`apply_action`](`Round::apply_action`).
///
/// ## Events
///
/// Certain actions trigger events, such as the end of a trick. These events are stored in a queue,
/// which may be drained using [`pop_event`](`Round::pop_event`).
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
        let contract = self.contract()?;
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
    /// The dealer for this round.
    dealer: Seat,
    /// Each player's hand, as dealt.
    hands: HashMap<Seat, Vec<Card>>,
    /// The upturned card, as dealt.
    top: Card,
}

impl Distribution<RoundConfig> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RoundConfig {
        RoundConfig::new(rng.gen(), rng.gen()).expect("deck is valid")
    }
}

impl RoundConfig {
    /// Creates a new [`RoundConfig`], with the specified dealer & deck.
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
        let mut round = Self { dealer, hands, top };
        round.validate()?;
        round.canonicalize();
        Ok(round)
    }

    /// Creates a [`RoundConfig`] with a random dealer and a shuffled deck.
    pub fn random() -> Self {
        rand::random()
    }

    /// Creates a specified dealer and a shuffled deck.
    pub fn random_with_dealer(dealer: Seat) -> Self {
        let deck = rand::random();
        Self::new(dealer, deck).expect("deck is valid")
    }

    /// Returns the dealer for this round.
    pub fn dealer(&self) -> Seat {
        self.dealer
    }

    /// Validates and canonicalizes the configuration.
    pub fn validate(&self) -> Result<(), RoundError> {
        let mut seen: HashSet<_> = HashSet::with_capacity(21);
        seen.insert(self.top);
        for hand in self.hands.values() {
            if hand.len() != 5 {
                return Err(RoundError::InvalidHandSize);
            }
            seen.extend(hand);
        }
        if seen.len() == 21 {
            Ok(())
        } else {
            Err(RoundError::DuplicateCard)
        }
    }

    /// Canonicalizes the configuration.
    pub fn canonicalize(&mut self) {
        for hand in self.hands.values_mut() {
            hand.sort_unstable_by_key(|c| (c.suit, c.rank));
        }
    }
}

/// The contract established by whomever calls suit.
#[derive(Debug, Clone, Copy)]
pub struct Contract {
    pub maker: Seat,
    pub suit: Suit,
    pub alone: bool,
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
    /// Creates a new [`RoundOutcome`].
    pub fn new(team: Team, points: u8) -> Self {
        RoundOutcome { team, points }
    }
}

/// The state visible to a particular seat.
#[derive(Debug)]
pub struct PlayerState<'a> {
    /// The player who has access to this state.
    pub seat: Seat,
    /// The dealer of this round.
    pub dealer: Seat,
    /// The top card for this round.
    pub top: Card,
    /// The contract for this round, if one has been declared.
    pub contract: Option<Contract>,
    /// The player's hand.
    pub hand: &'a Vec<Card>,
    /// The tricks played so far this round.
    pub tricks: &'a Tricks,
}

impl<'a> PlayerState<'a> {
    /// Creates a new [`PlayerState`].
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

    /// Returns the player's hand, in sorted order, based on effective suit and
    /// intrinsic card value.
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
