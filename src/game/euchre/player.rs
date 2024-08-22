use super::{ActionData, ActionType, Card, Event, PlayerError, PlayerState, Suit, Trick};

mod console;
mod robot;
pub use console::Console;
pub use robot::Robot;

/// A trait that implements a euchre player.
pub trait Player {
    /// Take the specified action.
    fn take_action(&self, state: PlayerState, action: ActionType) -> ActionData;

    /// Indicates that the player has made an invalid play.
    ///
    /// The implementation may return true, if a retry is desired. Otherwise,
    /// the invalid play will be converted into a fatal error.
    #[allow(unused_variables)]
    fn handle_error(&self, err: PlayerError) -> bool {
        false
    }

    /// Notifies the player of a public event.
    #[allow(unused_variables)]
    fn notify(&self, state: PlayerState, event: &Event) {}
}
