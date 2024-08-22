use std::{fmt::Display, str::FromStr};

use ansi_term::ANSIString;
use ratatui::text::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn to_span(self) -> Span<'static> {
        use ratatui::style::Color;
        match self {
            Suit::Club | Suit::Spade => Span::raw(self.to_string()),
            Suit::Diamond | Suit::Heart => Span::raw(self.to_string()).style(Color::Red),
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
