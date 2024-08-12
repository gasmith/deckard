//! The game of euchre.
//!
//! Todo:
//!
//!  - Feature flag for no-trump
//!  - Feature flag for stick-the-dealer

use std::collections::HashMap;

use itertools::iproduct;
use rand::prelude::*;

mod card;
mod player;
mod round;
mod trick;
#[cfg(feature = "tui")]
mod tui;
pub use card::{Card, Rank, Suit};
pub use player::Player;
use player::Players;
use round::{Outcome, Round};
pub use trick::Trick;
#[cfg(feature = "tui")]
pub use tui::tui_main;

use self::player::{Console, Robot};

/// Table position, represented as cardinal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    North,
    East,
    South,
    West,
}
impl Dir {
    fn all_dirs() -> &'static [Dir; 4] {
        static DIRS: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];
        &DIRS
    }

    #[cfg(test)]
    fn from_char(s: char) -> Option<Self> {
        let dir = match s {
            'N' => Dir::North,
            'E' => Dir::East,
            'S' => Dir::South,
            'W' => Dir::West,
            _ => return None,
        };
        Some(dir)
    }

    fn opposite(self) -> Dir {
        match self {
            Dir::North => Dir::South,
            Dir::East => Dir::West,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
        }
    }

    fn next(self) -> Dir {
        match self {
            Dir::North => Dir::East,
            Dir::East => Dir::South,
            Dir::South => Dir::West,
            Dir::West => Dir::North,
        }
    }

    fn next_n(mut self, n: usize) -> Vec<Dir> {
        let mut order = vec![];
        for _ in 0..n {
            self = self.next();
            order.push(self);
        }
        order
    }
}

/// Team specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    NorthSouth,
    EastWest,
}
impl From<Dir> for Team {
    fn from(value: Dir) -> Self {
        match value {
            Dir::North | Dir::South => Team::NorthSouth,
            Dir::East | Dir::West => Team::EastWest,
        }
    }
}

impl Team {
    fn other(self) -> Team {
        match self {
            Team::NorthSouth => Team::EastWest,
            Team::EastWest => Team::NorthSouth,
        }
    }
}

pub struct Deck {
    cards: Vec<Card>,
}
impl Default for Deck {
    fn default() -> Self {
        let cards = iproduct!(Rank::all_ranks(), Suit::all_suits())
            .map(|(&rank, &suit)| Card { rank, suit })
            .collect();
        Self { cards }
    }
}
impl Deck {
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

/// A euchre game consists of a set of players.
struct Euchre {
    players: Players,
    next_dealer: Dir,
    points: HashMap<Team, u8>,
}

impl Euchre {
    pub fn new(players: Players) -> Self {
        Self {
            players,
            next_dealer: Dir::North,
            points: HashMap::from([(Team::NorthSouth, 0), (Team::EastWest, 0)]),
        }
    }

    fn run_round<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<Outcome, Error> {
        let mut deck = Deck::default();
        deck.shuffle(rng);
        let round = Round::new(self.next_dealer, deck);
        let outcome = round.run(&self.players)?;
        let points = self.points.get_mut(&outcome.team).expect("init points");
        notify(&self.players, Event::Round(outcome.clone()));
        *points += outcome.points;
        self.next_dealer = self.next_dealer.next();
        println!("Standings: {:?}", self.points);
        Ok(outcome)
    }

    #[allow(dead_code)]
    fn winner(&self) -> Option<Team> {
        for (team, points) in &self.points {
            if *points >= 10 {
                return Some(*team);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Bid(Contract),
    Trick(Trick),
    Round(Outcome),
}

#[derive(Debug, Clone, Copy)]
pub struct Contract {
    pub maker: Dir,
    pub suit: Suit,
    pub alone: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum InvalidPlay {
    /// The dealer is required to choose a suit after all players have passed.
    DealerMustBid,

    /// Cannot bid the same suit as the top card.
    CannotBidTopSuit,

    /// The player doesn't actually hold the card they attempted to play.
    CardNotHeld,

    /// The player must follow the lead card for this trick.
    MustFollowLead,
}

#[derive(Debug, Clone, Copy)]
enum Error {
    #[allow(dead_code)]
    InvalidPlay(Dir, InvalidPlay),
}

/// Discards a card from the hand, returning true if the card was found.
fn discard(hand: &mut Vec<Card>, card: Card) -> bool {
    if let Some(index) = hand.iter().position(|c| *c == card) {
        hand.remove(index);
        true
    } else {
        false
    }
}

/// Notifies all players of an event.
fn notify(players: &Players, event: Event) {
    for player in players.iter() {
        player.notify(&event)
    }
}

pub fn cli_main() {
    let players = Players::new(
        Dir::all_dirs()
            .iter()
            .map(|&d| {
                let player = if d == Dir::South {
                    Console::new(d).into_player()
                } else {
                    Robot::new(d).into_player()
                };
                //let player = Robot::new(d).into_player();
                (d, player)
            })
            .collect(),
    );
    let mut euchre = Euchre::new(players);
    let mut rng = rand::thread_rng();
    loop {
        let _ = euchre.run_round(&mut rng).expect("robot malfunction");
        if let Some(team) = euchre.winner() {
            println!("{:?} wins!", team);
            break;
        }
    }
}
