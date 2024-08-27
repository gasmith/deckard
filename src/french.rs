//! French deck

use std::fmt::Display;
use std::str::FromStr;

use ansi_term::ANSIString;
use ratatui::text::Span;
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

    /// Parses a suit from a character.
    pub fn from_char(s: char) -> Option<Self> {
        let suit = match s {
            '♣' | '♧' | 'C' | 'c' => Suit::Club,
            '♥' | '♡' | 'H' | 'h' => Suit::Heart,
            '♦' | '♢' | 'D' | 'd' => Suit::Diamond,
            '♠' | '♤' | 'S' | 's' => Suit::Spade,
            _ => return None,
        };
        Some(suit)
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
            Suit::Heart => "♡",
            Suit::Spade => "♤",
        };
        f.write_str(sym)
    }
}
