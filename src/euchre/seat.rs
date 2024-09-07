//! Table position.

use std::{convert::TryFrom, fmt::Display};

use rand::distributions::{Distribution, Standard};
use serde::{Deserialize, Serialize};

/// Table position, represented as cardinal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Seat {
    North,
    East,
    South,
    West,
}
impl Display for Seat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Seat::North => "North",
            Seat::East => "East",
            Seat::South => "South",
            Seat::West => "West",
        })
    }
}
impl Distribution<Seat> for Standard {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> Seat {
        match rng.gen_range(0..=3) {
            0 => Seat::North,
            1 => Seat::East,
            2 => Seat::South,
            3 => Seat::West,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<char> for Seat {
    type Error = ();

    fn try_from(c: char) -> Result<Self, Self::Error> {
        Ok(match c {
            'N' | 'n' => Seat::North,
            'E' | 'e' => Seat::East,
            'S' | 's' => Seat::South,
            'W' | 'w' => Seat::West,
            _ => return Err(()),
        })
    }
}

impl Seat {
    /// All possible table positions, in clockwise order.
    pub fn all_seats() -> &'static [Seat; 4] {
        static SEATS: [Seat; 4] = [Seat::North, Seat::East, Seat::South, Seat::West];
        &SEATS
    }

    /// The team for this table position.
    pub fn team(self) -> Team {
        Team::from(self)
    }

    /// Returns an abbreviated name for the table position.
    pub fn to_abbr(self) -> char {
        match self {
            Seat::North => 'N',
            Seat::East => 'E',
            Seat::South => 'S',
            Seat::West => 'W',
        }
    }

    /// The opposite table position.
    pub fn opposite(self) -> Seat {
        match self {
            Seat::North => Seat::South,
            Seat::East => Seat::West,
            Seat::South => Seat::North,
            Seat::West => Seat::East,
        }
    }

    /// The next table position, in clockwise order.
    pub fn next(self) -> Seat {
        match self {
            Seat::North => Seat::East,
            Seat::East => Seat::South,
            Seat::South => Seat::West,
            Seat::West => Seat::North,
        }
    }

    /// The next N table positions in clockwise order.
    pub fn next_n(mut self, n: usize) -> Vec<Seat> {
        let mut order = vec![];
        for _ in 0..n {
            self = self.next();
            order.push(self);
        }
        order
    }
}

/// A team consists of the two seats opposite one another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    NorthSouth,
    EastWest,
}
impl From<Seat> for Team {
    fn from(value: Seat) -> Self {
        match value {
            Seat::North | Seat::South => Team::NorthSouth,
            Seat::East | Seat::West => Team::EastWest,
        }
    }
}
impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Team::NorthSouth => "North/South",
            Team::EastWest => "East/West",
        })
    }
}
impl Team {
    /// Returns an abbreviated name for the team.
    pub fn to_abbr(self) -> &'static str {
        match self {
            Team::NorthSouth => "N/S",
            Team::EastWest => "E/W",
        }
    }

    /// The other team.
    pub fn other(self) -> Team {
        match self {
            Team::NorthSouth => Team::EastWest,
            Team::EastWest => Team::NorthSouth,
        }
    }
}
