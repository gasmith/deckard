//! Error types

use super::{round::Id, ActionType, Card, Seat, Suit};

#[derive(Debug, Clone, thiserror::Error)]
pub enum PlayerError {
    /// The dealer is required to choose a suit after all players have passed.
    #[error("the dealer must bid")]
    DealerMustBidOther,

    /// Cannot bid the same suit as the top card.
    #[error("cannot bid {0}")]
    CannotBidTopSuit(Suit),

    /// The player doesn't actually hold the card they attempted to play.
    #[error("{0} does not hold {1}")]
    CardNotHeld(Seat, Card),

    /// The player must follow the lead card for this trick.
    #[error("{0} must follow {1}")]
    MustFollowLead(Seat, Card),
}

#[derive(Debug, thiserror::Error)]
pub enum RoundError {
    #[error("deck is missing cards")]
    IncompleteDeck,
    #[error("deck contains duplicate card")]
    DuplicateCard,
    #[error("action contains invalid data")]
    InvalidActionData,
    #[error("expected {seat} to {action}")]
    ExpectActioned { seat: Seat, action: ActionType },
    #[error("game over")]
    GameOver,
    #[error(transparent)]
    Player(#[from] PlayerError),
    #[error("invalid log id {0}")]
    InvalidLogId(Id),
}

