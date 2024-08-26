//! Tricks played during a round.

use std::convert::TryFrom;

use delegate::delegate;

use super::{Team, Trick};

/// Tricks played this round.
#[derive(Debug, Clone)]
pub struct Tricks {
    tricks: Vec<Trick>,
    trick_size: usize,
}

impl Default for Tricks {
    fn default() -> Self {
        Self {
            tricks: vec![],
            trick_size: 4,
        }
    }
}

impl Tricks {
    delegate! {
        to self.tricks {
            pub fn len(&self) -> usize;
            pub fn last(&self) -> Option<&Trick>;
            pub fn last_mut(&mut self) -> Option<&mut Trick>;
        }
    }

    /// Creates a new trick.
    pub fn push(&mut self, trick: Trick) {
        assert!(self.len() < 5);
        self.tricks.push(trick);
    }

    /// Returns the number of cards in each trick.
    pub fn trick_size(&self) -> usize {
        self.trick_size
    }

    /// Sets the number of cards in each trick.
    pub fn set_trick_size(&mut self, trick_size: usize) {
        assert!(matches!(trick_size, 3 | 4));
        self.trick_size = trick_size;
    }

    /// Counts the number of completed tricks won by the specified team.
    pub fn win_count(&self, team: Team) -> u8 {
        let count = self
            .tricks
            .iter()
            .filter(|t| t.len() == self.trick_size && Team::from(t.best().0) == team)
            .count();
        u8::try_from(count).expect("less than 256")
    }
}
