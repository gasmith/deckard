//! Euchre deck.

use std::convert::{TryFrom, TryInto};
use std::{fmt::Display, str::FromStr};

use ansi_term::ANSIString;
use ratatui::text::Span;
use serde::{Deserialize, Serialize};

use crate::deck;
use crate::french;
pub use crate::french::Suit;

/// Euchre card rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rank {
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}
impl From<Rank> for french::Rank {
    fn from(r: Rank) -> Self {
        match r {
            Rank::Nine => french::Rank::Nine,
            Rank::Ten => french::Rank::Ten,
            Rank::Jack => french::Rank::Jack,
            Rank::Queen => french::Rank::Queen,
            Rank::King => french::Rank::King,
            Rank::Ace => french::Rank::Ace,
        }
    }
}
impl TryFrom<french::Rank> for Rank {
    type Error = ();

    fn try_from(r: french::Rank) -> Result<Self, Self::Error> {
        Ok(match r {
            french::Rank::Nine => Rank::Nine,
            french::Rank::Ten => Rank::Ten,
            french::Rank::Jack => Rank::Jack,
            french::Rank::Queen => Rank::Queen,
            french::Rank::King => Rank::King,
            french::Rank::Ace => Rank::Ace,
            _ => return Err(()),
        })
    }
}
impl TryFrom<char> for Rank {
    type Error = ();

    fn try_from(c: char) -> Result<Self, Self::Error> {
        french::Rank::try_from(c)?.try_into()
    }
}

impl Rank {
    /// Returns an array of all ranks, in no particular order.
    pub fn all_ranks() -> &'static [Rank] {
        static RANKS: [Rank; 6] = [
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        &RANKS
    }
}
impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        french::Rank::from(*self).fmt(f)
    }
}

/// A euchre card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    /// Card rank.
    pub rank: Rank,
    /// Card suit.
    pub suit: Suit,
}
impl From<Card> for french::Card {
    fn from(card: Card) -> Self {
        french::Card {
            rank: card.rank.into(),
            suit: card.suit,
        }
    }
}
impl TryFrom<french::Card> for Card {
    type Error = ();

    fn try_from(card: french::Card) -> Result<Self, Self::Error> {
        Ok(Card {
            rank: card.rank.try_into()?,
            suit: card.suit,
        })
    }
}
impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        french::Card::from(*self).fmt(f)
    }
}
impl FromStr for Card {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        french::Card::from_str(s)?.try_into()
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
        let card = french::Card::deserialize(deserializer)?;
        card.try_into()
            .map_err(|()| serde::de::Error::custom("not a euchre card"))
    }
}
impl Card {
    /// Creates a new [`Card`].
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }

    /// Returns a string representation of the card, decorated with ANSI color codes.
    pub fn to_ansi_string(self) -> ANSIString<'static> {
        use ansi_term::Colour::Red;
        match self.suit {
            Suit::Club | Suit::Spade => self.to_string().into(),
            Suit::Diamond | Suit::Heart => Red.paint(self.to_string()),
        }
    }

    /// Returns a [`ratatui::text::Span`] for the card.
    pub fn to_span(self) -> Span<'static> {
        use ratatui::style::Color;
        match self.suit {
            Suit::Club | Suit::Spade => Span::raw(self.to_string()),
            Suit::Diamond | Suit::Heart => Span::raw(self.to_string()).style(Color::Red),
        }
    }

    /// Returns true if the card is consindered to be trump, given the suit declared in the
    /// contract.
    pub fn is_trump(self, trump: Suit) -> bool {
        self.suit == trump || matches!(self.rank, Rank::Jack) && self.suit.color() == trump.color()
    }

    /// Returns the effective suit for this card, given the suit declared in the contract.
    pub fn effective_suit(self, trump: Suit) -> Suit {
        if self.is_trump(trump) {
            trump
        } else {
            self.suit
        }
    }

    /// Returns true if the played card is the same effective suit as the card that was lead.
    pub fn is_following(self, trump: Suit, lead: Card) -> bool {
        self.effective_suit(trump) == lead.effective_suit(trump)
    }

    /// Returns the value of the card, for determining the winner of a trick.
    pub fn value(self, trump: Suit, lead: Card) -> u8 {
        if self.is_trump(trump) {
            match self.rank {
                Rank::Nine => 7,
                Rank::Ten => 8,
                Rank::Queen => 9,
                Rank::King => 10,
                Rank::Ace => 11,
                Rank::Jack => {
                    if self.suit == trump {
                        13
                    } else {
                        12
                    }
                }
            }
        } else if self.suit == lead.suit && !lead.is_trump(trump) {
            match self.rank {
                Rank::Nine => 1,
                Rank::Ten => 2,
                Rank::Jack => 3,
                Rank::Queen => 4,
                Rank::King => 5,
                Rank::Ace => 6,
            }
        } else {
            0
        }
    }
}

/// A euchre deck.
pub type Deck = deck::Deck<Card>;
impl Default for Deck {
    fn default() -> Self {
        itertools::iproduct!(Rank::all_ranks(), Suit::all_suits())
            .map(|(&rank, &suit)| Card { rank, suit })
            .collect()
    }
}
