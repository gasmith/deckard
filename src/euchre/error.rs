//! Errors

use super::{ActionType, Card, LogId, Seat, Suit};

/// An invalid action taken by a player.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlayerError {
    /// The dealer is required to choose a suit after all players have passed.
    #[error("the dealer must bid")]
    DealerMustBidOther,

    /// Must call the same suit as the top card.
    #[error("must call {0}")]
    MustCallTopSuit(Suit),

    /// Cannot call the same suit as the top card.
    #[error("cannot call {0}")]
    CannotCallTopSuit(Suit),

    /// The player doesn't actually hold the card they attempted to play.
    #[error("{0} does not hold {1}")]
    CardNotHeld(Seat, Card),

    /// The player must follow the lead card for this trick.
    #[error("{0} must follow {1}")]
    MustFollowLead(Seat, Card),
}

/// An error that can occur during the round.
#[derive(Debug, thiserror::Error)]
pub enum RoundError {
    /// Not playing with a full deck.
    #[error("deck is missing cards")]
    IncompleteDeck,
    /// The deck has duplicate cards.
    #[error("deck contains duplicate card")]
    DuplicateCard,
    /// A player has too many or too few cards.
    #[error("a player has the incorrect number of cards")]
    InvalidHandSize,
    /// The provided [`ActionData`](super::ActionData) is not appropriate for the [`ActionType`].
    #[error("action contains invalid data")]
    InvalidActionData,
    /// The provided [`Action`](super::Action) doesn't match the expected [`ExpectAction`](super::ExpectAction).
    #[error("expected {seat} to {action}")]
    ExpectActioned { seat: Seat, action: ActionType },
    /// The game is over, no more actions are expected.
    #[error("round is over")]
    RoundOver,
    /// Invalid reference to a log record.
    #[error("invalid log id {0}")]
    InvalidLogId(LogId),
    /// A player attempted to play an invalid action.
    #[error(transparent)]
    Player(#[from] PlayerError),
}
