use std::{fmt::Display, str::FromStr};

use ansi_term::ANSIString;
use itertools::iproduct;
use rand::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Club,
    Diamond,
    Spade,
    Heart,
}
impl Suit {
    pub fn all_suits() -> &'static [Suit] {
        static SUITS: [Suit; 4] = [Suit::Club, Suit::Diamond, Suit::Heart, Suit::Spade];
        &SUITS
    }

    pub fn to_ansi_string(self) -> ANSIString<'static> {
        use ansi_term::Colour::Red;
        match self {
            Suit::Club | Suit::Spade => self.to_string().into(),
            Suit::Diamond | Suit::Heart => Red.paint(self.to_string()),
        }
    }

    pub fn from_char(s: char) -> Option<Self> {
        let suit = match s {
            '♣' | 'C' | 'c' => Suit::Club,
            '♦' | 'D' | 'd' => Suit::Diamond,
            '♥' | 'H' | 'h' => Suit::Heart,
            '♠' | 'S' | 's' => Suit::Spade,
            _ => return None,
        };
        Some(suit)
    }

    pub fn color(self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }

    pub fn to_matching_color(self) -> Self {
        match self {
            Suit::Club => Suit::Spade,
            Suit::Diamond => Suit::Heart,
            Suit::Heart => Suit::Diamond,
            Suit::Spade => Suit::Club,
        }
    }
}

impl FromStr for Suit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let suit = chars.next().and_then(Suit::from_char).ok_or(())?;
        if chars.next().is_none() {
            Ok(suit)
        } else {
            Err(())
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

    pub fn from_char(s: char) -> Option<Self> {
        let suit = match s {
            'A' | 'a' => Rank::Ace,
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' | 't' => Rank::Ten,
            'J' | 'j' => Rank::Jack,
            'Q' | 'q' => Rank::Queen,
            'K' | 'k' => Rank::King,
            _ => return None,
        };
        Some(suit)
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
