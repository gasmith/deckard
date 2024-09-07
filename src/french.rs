//! French deck

use std::convert::TryInto;
use std::fmt::{self, Write};
use std::str::FromStr;
use std::{convert::TryFrom, fmt::Display};

use ansi_term::ANSIString;
use ratatui::text::Span;
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};

/// Suit color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

/// French suits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Suit {
    Club,
    Diamond,
    Spade,
    Heart,
}

impl Suit {
    /// Returns an array of all suits, in no particular order.
    pub fn all_suits() -> &'static [Suit] {
        static SUITS: [Suit; 4] = [Suit::Club, Suit::Diamond, Suit::Heart, Suit::Spade];
        &SUITS
    }

    /// Returns the color of this suit.
    pub fn color(self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }

    /// Returns a string representation of the suit, decorated with ANSI color codes.
    pub fn to_ansi_string(self) -> ANSIString<'static> {
        use ansi_term::Colour::Red;
        match self.color() {
            Color::Black => self.to_string().into(),
            Color::Red => Red.paint(self.to_string()),
        }
    }

    /// Returns a [`ratatui::text::Span`] for the suit.
    pub fn to_span(self) -> Span<'static> {
        use ratatui::style::Color::Red;
        match self.color() {
            Color::Black => Span::raw(self.to_string()),
            Color::Red => Span::raw(self.to_string()).style(Red),
        }
    }

    /// Returns the other suit of the same color.
    pub fn to_matching_color(self) -> Self {
        match self {
            Suit::Club => Suit::Spade,
            Suit::Diamond => Suit::Heart,
            Suit::Heart => Suit::Diamond,
            Suit::Spade => Suit::Club,
        }
    }
}

impl TryFrom<char> for Suit {
    type Error = ();
    fn try_from(s: char) -> Result<Self, ()> {
        let suit = match s {
            '♣' | '♧' | 'C' | 'c' => Suit::Club,
            '♥' | '♡' | 'H' | 'h' => Suit::Heart,
            '♦' | '♢' | 'D' | 'd' => Suit::Diamond,
            '♠' | '♤' | 'S' | 's' => Suit::Spade,
            _ => return Err(()),
        };
        Ok(suit)
    }
}

impl FromStr for Suit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let suit = chars.next().ok_or(())?.try_into()?;
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
            Suit::Club => '♣',
            Suit::Diamond => '♦',
            Suit::Heart => '♡',
            Suit::Spade => '♤',
        };
        f.write_char(sym)
    }
}

/// French ranks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        };
        f.write_char(sym)
    }
}

impl TryFrom<char> for Rank {
    type Error = ();
    fn try_from(s: char) -> Result<Self, ()> {
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
            _ => return Err(()),
        };
        Ok(suit)
    }
}

impl FromStr for Rank {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let rank = chars.next().ok_or(())?.try_into()?;
        if chars.next().is_none() {
            Ok(rank)
        } else {
            Err(())
        }
    }
}

pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

impl FromStr for Card {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let rank: Rank = chars.next().ok_or(())?.try_into()?;
        let suit: Suit = chars.next().ok_or(())?.try_into()?;
        if chars.next().is_none() {
            Ok(Card { rank, suit })
        } else {
            Err(())
        }
    }
}

impl Serialize for Card {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CardVisitor;
        impl<'de> Visitor<'de> for CardVisitor {
            type Value = Card;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a card")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Card::from_str(value).map_err(|()| E::custom("invalid card"))
            }
        }
        deserializer.deserialize_str(CardVisitor)
    }
}
