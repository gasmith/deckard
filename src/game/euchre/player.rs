use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::ops::Index;
use std::sync::Arc;

use super::{Bid, Card, Contract, Dir, Event, InvalidPlay, Suit, Trick};

#[cfg(test)]
mod scripted;
#[cfg(test)]
pub use scripted::ScriptedPlayer;

/// A trait that implements a euchre player.
pub trait Player {
    /// The `dealer` deals a new hand of `cards` to this player, and reveals
    /// the top card.
    fn deal(&self, dealer: Dir, cards: Vec<Card>, top: Card);

    /// This player is allowed to bid on the suit displayed on the upturned
    /// card. All preceding players seated clockwise from the dealer have
    /// passed.
    ///
    /// If this function returns true, the player is accepting a contract to
    /// win 3 or more tricks, and the card will go into the dealer's hand.
    fn bid_top(&self, dealer: Dir, top: Card) -> Option<Contract>;

    /// This player is allowed to bid on any other suit other than that of the
    /// upturned card offered in [`bid_top`]. All preceding players seated
    /// clockwise from the dealer have passed.
    ///
    /// The dealer is required to bid.
    fn bid_other(&self, dealer: Dir) -> Option<(Suit, Contract)>;

    /// The dealer takes up the top card, and discards a card. The card must
    /// come from the player's hand.
    fn pick_up_top(&self, card: Card, bid: Bid) -> Card;

    /// Leads a new trick. The card must come from the player's hand.
    fn lead_trick(&self) -> Card;

    /// Plays a card into an opened trick. The card must come from the player's
    /// hand. The player's card must follow the lead suit when possible.
    fn follow_trick(&self, trick: &Trick) -> Card;

    /// A notification of an event that all players can see.
    fn notify(&self, event: &Event);

    /// Indicates that the player has made an invalid play.
    ///
    /// The implementation may return true, if a retry is desired. Otherwise,
    /// the invalid play will be converted into a fatal error.
    fn invalid_play(&self, invalid: InvalidPlay) -> bool;
}

/// A collection of players, indexed by table position.
pub struct Players(HashMap<Dir, Arc<dyn Player>>);

impl Index<Dir> for Players {
    type Output = Arc<dyn Player>;
    fn index(&self, index: Dir) -> &Self::Output {
        self.0.get(&index).expect("all players present")
    }
}

impl Players {
    pub fn new(players: HashMap<Dir, Arc<dyn Player>>) -> Self {
        Self(players)
    }

    /// Returns an iterator over players, in arbitrary order.
    pub fn iter(&self) -> Values<'_, Dir, Arc<dyn Player>> {
        self.0.values()
    }
}
