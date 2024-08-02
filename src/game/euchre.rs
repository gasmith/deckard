use std::{collections::HashMap, ops::Index, sync::Arc};

use crate::french::{Card, Rank, Suit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    North,
    East,
    South,
    West,
}

impl Dir {
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
            order.push(self);
            self = self.next();
        }
        order
    }
}

struct Players(HashMap<Dir, Arc<dyn Player>>);
impl Index<Dir> for Players {
    type Output = Arc<dyn Player>;

    fn index(&self, index: Dir) -> &Self::Output {
        self.0.get(&index).expect("all players present")
    }
}

struct Game {
    players: Players,
    ns_points: u8,
    ew_points: u8,
}

#[derive(Debug)]
enum Round {
    Deal(Deal),
    Play(Play),
}
#[derive(Debug)]
struct Deal {
    hands: HashMap<Dir, Hand>,
    dealer: Dir,
    top: Card,
}
#[derive(Debug)]
struct Play {
    hands: HashMap<Dir, Hand>,
    trump: Suit,
    dealer: Dir,
    maker: Dir,
}

#[derive(Debug)]
struct Hand {
    cards: Vec<Card>,
    tricks: Vec<Trick>,
}

#[derive(Debug)]
pub struct Trick {
    pub trump: Suit,
    pub cards: Vec<(Dir, Card)>,
}

#[derive(Debug)]
pub enum Event {
    InvalidPlay(String),
    TrumpDeclared(Dir, Suit),
    TrickComplete(Dir, Trick),
}

pub trait Player {
    fn deal(&self, dealer: Dir, cards: Vec<Card>);
    fn bid_top(&self, dealer: Dir, top: Card) -> bool;
    fn bid_other(&self, dealer: Dir) -> Option<Suit>;
    fn pick_up(&self, dir: Dir, trump: Suit) -> Card;
    fn lead(&self) -> Card;
    fn follow(&self, trick: &Trick) -> Card;
    fn event(&self, event: Event);
}

pub fn card_is_trump(trump: Suit, card_suit: Suit, card_rank: Rank) -> bool {
    card_suit == trump || matches!(card_rank, Rank::Jack) && card_suit.color() == trump.color()
}

pub fn card_value(
    trump: Suit,
    lead_suit: Suit,
    lead_rank: Rank,
    card_suit: Suit,
    card_rank: Rank,
) -> u8 {
    if card_is_trump(trump, card_suit, card_rank) {
        match card_rank {
            Rank::Nine => 7,
            Rank::Ten => 8,
            Rank::Queen => 9,
            Rank::King => 10,
            Rank::Ace => 11,
            Rank::Jack => {
                if card_suit == trump {
                    13
                } else {
                    12
                }
            }
            _ => unreachable!(),
        }
    } else if card_suit == lead_suit && !card_is_trump(trump, lead_suit, lead_rank) {
        match card_rank {
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

enum Bid {
    Top(Dir),
    Other(Dir, Suit),
}

impl Deal {
    fn bid(self, players: &Players) -> Bid {
        let mut dir = self.dealer.next();
        for _ in 0..4 {
            let player = &players[dir];
            let hand = self.hands.get(&dir).expect("hands");
            player.deal(self.dealer, hand.cards.clone());
            dir = dir.next();
        }
        for _ in 0..4 {
            let player = &players[dir];
            if player.bid_top(self.dealer, self.top) {
                return Bid::Top(dir);
            }
            dir = dir.next();
        }
        for _ in 0..4 {
            let player = &players[dir];
            // Need to validate that the suit doesn't match the top card.
            // Probably ought to move this around, and force the player to call
            // back into the game, so that we can do validation inside the
            // callback, rather than validating the return value.
            if let Some(card) = player.bid_other(self.dealer) {
                return Bid::Other(dir, card);
            }
            dir = dir.next();
        }
        let player = &players[self.dealer];
        loop {
            // Need more nuanced types for reporting invalid play.
            player.event(Event::InvalidPlay("Dealer must choose a suit".into()));
            if let Some(card) = player.bid_other(self.dealer) {
                return Bid::Other(self.dealer, card);
            }
        }
    }
}

impl Trick {
    fn new(leader: Dir, card: Card, trump: Suit) -> Self {
        Self {
            trump,
            cards: vec![(leader, card)],
        }
    }

    fn leader(&self) -> Dir {
        self.cards[0].0
    }

    fn lead(&self) -> Card {
        self.cards[0].1
    }

    fn winner(&self) -> Option<Dir> {
        assert!(!self.cards.is_empty()); // by construction
        let Card::RankSuit(lead_rank, lead_suit) = self.lead() else {
            panic!("no jokers!");
        };
        let dir = self
            .cards
            .iter()
            .max_by_key(|(_, card)| {
                let Card::RankSuit(rank, suit) = card else {
                    panic!("no jokers!");
                };
                card_value(self.trump, lead_suit, lead_rank, *suit, *rank)
            })
            .expect("non-empty")
            .0;
        Some(dir)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn trick(trump: char, cards: &[&str]) -> Trick {
        let trump = Suit::from_char(trump).unwrap();
        let cards = cards
            .iter()
            .map(|s| {
                let mut chars = s.chars();
                let dir = chars.next().and_then(Dir::from_char).unwrap();
                let rank = chars.next().and_then(Rank::from_char).unwrap();
                let suit = chars.next().and_then(Suit::from_char).unwrap();
                assert!(chars.next().is_none());
                (dir, Card::RankSuit(rank, suit))
            })
            .collect();
        Trick { trump, cards }
    }

    #[test]
    fn test_trick_winner() {
        assert_eq!(trick('♠', &["N9♥"]).winner(), Some(Dir::North));
        assert_eq!(
            trick('♠', &["N9♥", "ET♥", "SJ♥", "WQ♥"]).winner(),
            Some(Dir::West)
        );
        assert_eq!(
            trick('♠', &["NJ♠", "EK♠", "SA♣", "WJ♣"]).winner(),
            Some(Dir::North)
        );
        assert_eq!(
            trick('♠', &["NQ♠", "EK♠", "SA♣", "WJ♣"]).winner(),
            Some(Dir::West)
        );
    }
}
