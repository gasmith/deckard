use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{Card, Seat, Suit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    BidTop,
    BidOther,
    DealerDiscard,
    Lead,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionData {
    Pass,
    Call { suit: Suit, alone: bool },
    Card { card: Card },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub seat: Seat,
    pub action: ActionType,
    pub data: ActionData,
}

impl Action {
    pub fn new(seat: Seat, action: ActionType, data: ActionData) -> Self {
        Self { seat, action, data }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExpectAction {
    pub seat: Seat,
    pub action: ActionType,
}

impl ExpectAction {
    pub fn new(seat: Seat, action: ActionType) -> Self {
        Self { seat, action }
    }

    pub fn with_data(self, data: ActionData) -> Action {
        Action::new(self.seat, self.action, data)
    }
}
