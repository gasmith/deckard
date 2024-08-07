use std::fmt::Display;

pub use crate::french::Suit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rank {
    Ace,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}
impl Rank {
    pub fn all_ranks() -> &'static [Rank] {
        static RANKS: [Rank; 6] = [
            Rank::Ace,
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
            'A' => Rank::Ace,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            _ => return None,
        };
        Some(suit)
    }
}
impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self {
            Rank::Ace => "A",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        };
        f.write_str(sym)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn is_trump(&self, trump: Suit) -> bool {
        self.suit == trump || matches!(self.rank, Rank::Jack) && self.suit.color() == trump.color()
    }

    pub fn is_following(&self, trump: Suit, lead: Card) -> bool {
        match (self.is_trump(trump), lead.is_trump(trump)) {
            (true, true) => true,
            (true, false) | (false, true) => false,
            (false, false) => self.suit == lead.suit,
        }
    }

    pub fn value(&self, trump: Suit, lead: Card) -> u8 {
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
                _ => unreachable!(),
            }
        } else if self.suit == lead.suit && !lead.is_trump(trump) {
            match self.rank {
                Rank::Nine => 1,
                Rank::Ten => 2,
                Rank::Jack => 3,
                Rank::Queen => 4,
                Rank::King => 5,
                Rank::Ace => 6,
                _ => unreachable!(),
            }
        } else {
            0
        }
    }
}
