//! The game of euchre.
//!
//! Todo:
//!
//!  - Feature flag for no-trump
//!  - Feature flag for stick-the-dealer

use std::{collections::HashMap, ops::Index, sync::Arc};

mod card;
pub use card::{Card, Rank, Suit};

/// Table position, represented as cardinal direction.
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

/// A collection of players, indexed by table position.
struct Players(HashMap<Dir, Arc<dyn Player>>);

impl Index<Dir> for Players {
    type Output = Arc<dyn Player>;
    fn index(&self, index: Dir) -> &Self::Output {
        self.0.get(&index).expect("all players present")
    }
}

/// A game consists of a set of players.
struct Game {
    players: Players,
    next_dealer: Dir,
    ns_points: u8,
    ew_points: u8,
}

/// The main state machine for the round.
///
/// A new round is initiated by a deal into the `BidTop` state. The players
/// then bid for the top card. If a bid is made, the top card is replaces a
/// card in the dealer's hand, and the round progresses to `Play`.
///
/// Otherwise, the state machine advances to `BidOther`, where players bid for
/// any suit other than that of the top card. The dealer is required to choose
/// a suit, if no other player will.
#[derive(Debug)]
enum Round {
    Deal(Deal),
    BidTop(Deal),
    BidOther(Deal),
    Play(Play),
}

impl Round {
    fn next(self, players: &Players) -> Result<Self, Error> {
        match self {
            Round::Deal(deal) => {
                deal.deal(players);
                Ok(Round::BidTop(deal))
            }
            Round::BidTop(deal) => match deal.bid_top(players)? {
                Some(bid) => {
                    deal.notify(players, Event::Bid(bid));
                    deal.dealer_pick_up_top(players, bid).map(Round::Play)
                }
                None => Ok(Round::BidOther(deal)),
            },
            Round::BidOther(deal) => {
                let bid = deal.bid_other(players)?;
                deal.notify(players, Event::Bid(bid));
                Ok(Round::Play(deal.into_play(bid)))
            }
            Round::Play(_) => todo!(),
        }
    }
}

#[derive(Debug)]
struct Deal {
    hands: HashMap<Dir, Vec<Card>>,
    dealer: Dir,
    top: Card,
}
#[derive(Debug)]
struct Play {
    hands: HashMap<Dir, Vec<Card>>,
    tricks: HashMap<Dir, Vec<Trick>>,
    dealer: Dir,
    bid: Bid,
}

#[derive(Debug, Clone)]
pub struct Trick {
    pub trump: Suit,
    pub cards: Vec<(Dir, Card)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Bid(Bid),
    Trick(Dir, Trick),
}

pub trait Player {
    /// The `dealer` deals a new hand of `cards` to this player, and reveals
    /// the top card.
    fn deal(&self, dealer: Dir, cards: Vec<Card>, top: Card);

    /// This player is allowed to bid on the suit displayed on the upturned
    /// card. All preceding players seated clockwise from the dealer have
    /// passed.
    ///
    /// If this function returns true, the player is accepting a contract to
    /// win 3 or more tricks, and the card will go into the dealer's hand.
    fn bid_top(&self, dealer: Dir, top: Card) -> Option<Contract>;

    /// This player is allowed to bid on any other suit other than that of the
    /// upturned card offered in [`bid_top`]. All preceding players seated
    /// clockwise from the dealer have passed.
    ///
    /// The dealer is required to bid.
    fn bid_other(&self, dealer: Dir) -> Option<(Suit, Contract)>;

    /// The dealer takes up the top card, and discards a card. The card must
    /// come from the player's hand.
    fn pick_up_top(&self, card: Card, bid: Bid) -> Card;

    /// Leads a new trick. The card must come from the player's hand.
    fn lead(&self) -> Card;

    /// Plays a card into a trick. The card must come from the player's hand.
    fn follow(&self, trick: &Trick) -> Card;

    /// A notification of an event that all players can see.
    fn notify(&self, event: &Event);

    /// Indicates that the player has made an invalid play.
    ///
    /// The implementation may return true, if a retry is desired. Otherwise,
    /// the invalid play will be converted into a fatal error.
    fn invalid_play(&self, invalid: InvalidPlay) -> bool;
}

#[derive(Debug, Clone, Copy)]
enum Contract {
    Partner,
    Alone,
}

#[derive(Debug, Clone, Copy)]
struct Bid {
    dir: Dir,
    suit: Suit,
    contract: Contract,
}

#[derive(Debug, Clone, Copy)]
enum InvalidPlay {
    /// The dealer is required to choose a suit after all players have passed.
    DealerMustBid,

    /// The player doesn't actually hold the card they attempted to play.
    CardNotHeld,

    /// Cannot bid the same suit as the top card.
    CannotBidTopSuit,
}

#[derive(Debug, Clone, Copy)]
enum Error {
    InvalidPlay(Dir, InvalidPlay),
}

impl Deal {
    fn deal(&self, players: &Players) {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            let hand = self.hands.get(&dir).expect("hands");
            player.deal(self.dealer, hand.clone(), self.top);
        }
    }

    fn notify(&self, players: &Players, event: Event) {
        for dir in self.dealer.next_n(4) {
            players[dir].notify(&event)
        }
    }

    fn bid_top(&self, players: &Players) -> Result<Option<Bid>, Error> {
        let mut dir = self.dealer.next();
        for _ in 0..4 {
            let player = &players[dir];
            if let Some(contract) = player.bid_top(self.dealer, self.top) {
                let bid = Bid {
                    dir,
                    suit: self.top.suit,
                    contract,
                };
                return Ok(Some(bid));
            }
            dir = dir.next();
        }
        Ok(None)
    }

    fn dealer_pick_up_top(mut self, players: &Players, bid: Bid) -> Result<Play, Error> {
        let hand = self.hands.get_mut(&self.dealer).unwrap();
        hand.push(self.top);
        let dealer = &players[self.dealer];
        loop {
            let card = dealer.pick_up_top(self.top, bid);
            match hand.iter().position(|c| *c == card) {
                Some(index) => {
                    // Discard the card from the dealer's hand.
                    hand.remove(index);
                    break;
                }
                None => {
                    // The dealer attempted to discard a card they do not hold.
                    let invalid = InvalidPlay::CardNotHeld;
                    if !dealer.invalid_play(invalid) {
                        return Err(Error::InvalidPlay(self.dealer, invalid));
                    }
                }
            }
        }
        Ok(self.into_play(bid))
    }

    fn bid_other(&self, players: &Players) -> Result<Bid, Error> {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            loop {
                match player.bid_other(self.dealer) {
                    Some((suit, contract)) => {
                        if suit == self.top.suit {
                            let invalid = InvalidPlay::CannotBidTopSuit;
                            if !player.invalid_play(invalid) {
                                return Err(Error::InvalidPlay(dir, invalid));
                            }
                        }
                        return Ok(Bid {
                            dir,
                            suit,
                            contract,
                        });
                    }
                    None if dir == self.dealer => {
                        let invalid = InvalidPlay::DealerMustBid;
                        if !player.invalid_play(invalid) {
                            return Err(Error::InvalidPlay(dir, invalid));
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }
        unreachable!();
    }

    fn into_play(self, bid: Bid) -> Play {
        Play {
            hands: self.hands,
            tricks: HashMap::new(),
            dealer: self.dealer,
            bid,
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
        let dir = self
            .cards
            .iter()
            .max_by_key(|(_, card)| card.value(self.trump, self.lead()))
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
                (dir, Card { rank, suit })
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
