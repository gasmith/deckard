//! Trick management.

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
            pub fn push(&mut self, trick: Trick);
        }
    }

    pub fn set_trick_size(&mut self, trick_size: usize) {
        self.trick_size = trick_size;
    }

    pub fn win_count(&self, team: Team) -> u8 {
        self.tricks
            .iter()
            .filter(|t| t.len() == self.trick_size && Team::from(t.best().0) == team)
            .count() as u8
    }
}
