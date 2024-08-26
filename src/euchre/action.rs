//! Actions

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{Card, Seat, Suit};

/// Types of actions that a player can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// Bid the top card.
    BidTop,
    /// Bid a suit other than that of the top card.
    BidOther,
    /// Discard a card after picking up the top card as the dealer.
    DealerDiscard,
    /// Lead a new trick.
    Lead,
    /// Follow a pending trick.
    Follow,
}
impl Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ActionType::BidTop => "bid top",
            ActionType::BidOther => "bid other",
            ActionType::DealerDiscard => "discard",
            ActionType::Lead => "lead",
            ActionType::Follow => "follow",
        })
    }
}

/// The payload for actions that a player can take during the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionData {
    /// Pass on an opportunity to declare trump.
    Pass,

    /// Declare trump.
    Call {
        /// The suit to declare. If the action is [`ActionType::BidTop`], this must be the same
        /// suit as the top card. If the action is [`ActionType::BidOther`], it _must not_ be the
        /// same suit as the top card.
        suit: Suit,

        /// If true, the player's teammate will sit out for the rest of the round.
        alone: bool,
    },

    /// Play or discard a card.
    Card { card: Card },
}

/// The action that the game's state machine expects to happen next.
#[derive(Debug, Clone, Copy)]
pub struct ExpectAction {
    /// The player expected to take the action.
    pub seat: Seat,
    /// The type of action.
    pub action: ActionType,
}

impl ExpectAction {
    /// Create a new [`ExpectAction`].
    pub fn new(seat: Seat, action: ActionType) -> Self {
        Self { seat, action }
    }

    /// Bind in a payload to create an [`Action`].
    pub fn with_data(self, data: ActionData) -> Action {
        Action::new(self.seat, self.action, data)
    }
}

/// An action taken by a player during a round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// The player taking the action.
    pub seat: Seat,
    /// The type of action.
    pub action: ActionType,
    /// The action payload.
    pub data: ActionData,
}

impl Action {
    /// Create a new [`Action`].
    pub fn new(seat: Seat, action: ActionType, data: ActionData) -> Self {
        Self { seat, action, data }
    }
}
