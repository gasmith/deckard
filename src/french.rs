use std::fmt::Display;

use itertools::iproduct;
use rand::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Club,
    Diamond,
    Heart,
    Spade,
}
impl Suit {
    fn all_suits() -> &'static [Suit] {
        static SUITS: [Suit; 4] = [Suit::Club, Suit::Diamond, Suit::Heart, Suit::Spade];
        &SUITS
    }

    pub fn color(self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }
}
impl Display for Suit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self {
            Suit::Club => "♣",
            Suit::Diamond => "♦",
            Suit::Heart => "♥",
            Suit::Spade => "♠",
        };
        f.write_str(sym)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}
impl Rank {
    fn all_ranks() -> &'static [Rank] {
        static RANKS: [Rank; 13] = [
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ];
        &RANKS
    }
}
impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        };
        f.write_str(sym)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Card {
    RankSuit(Rank, Suit),
    Joker,
    BigJoker,
}
impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Card::RankSuit(r, s) => write!(f, "{r}{s}"),
            Card::Joker => write!(f, " 🃟"),
            Card::BigJoker => write!(f, "!🃟"),
        }
    }
}

#[derive(Default)]
pub struct Deck {
    cards: Vec<Card>,
}
impl Deck {
    pub fn standard() -> Self {
        let cards = iproduct!(Rank::all_ranks(), Suit::all_suits())
            .map(|(&r, &s)| Card::RankSuit(r, s))
            .collect();
        Self { cards }
    }

    pub fn with_cards<I: IntoIterator<Item = Card>>(mut self, cards: I) -> Self {
        self.cards.extend(cards);
        self
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn shuffle<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        self.cards.shuffle(rng);
    }

    pub fn take(&mut self, n: usize) -> Vec<Card> {
        let idx = self.cards.len().saturating_sub(n);
        self.cards.split_off(idx)
    }
}
