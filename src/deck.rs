//! A deck of cards.

use std::iter::FromIterator;

use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;

/// A deck of cards.
#[derive(Debug, Clone)]
pub struct Deck<C> {
    cards: Vec<C>,
}

impl<C> Distribution<Deck<C>> for Standard
where
    Deck<C>: Default,
{
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> Deck<C> {
        let mut deck = Deck::default();
        deck.cards.shuffle(rng);
        deck
    }
}

impl<C> FromIterator<C> for Deck<C> {
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        let cards = iter.into_iter().collect();
        Self { cards }
    }
}

impl<C> Deck<C> {
    /// The number of cards remaining in the deck.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Removes a card from the deck.
    pub fn take(&mut self, n: usize) -> Vec<C> {
        let idx = self.cards.len().saturating_sub(n);
        self.cards.split_off(idx)
    }
}
