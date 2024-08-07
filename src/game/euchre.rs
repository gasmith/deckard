//! The game of euchre.
//!
//! Todo:
//!
//!  - Feature flag for no-trump
//!  - Feature flag for stick-the-dealer

use std::collections::{hash_map::Values, HashMap};
use std::ops::Index;
use std::sync::Arc;

use itertools::iproduct;
use rand::prelude::*;

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

/// A collection of players, indexed by table position.
struct Players(HashMap<Dir, Arc<dyn Player>>);

impl Index<Dir> for Players {
    type Output = Arc<dyn Player>;
    fn index(&self, index: Dir) -> &Self::Output {
        self.0.get(&index).expect("all players present")
    }
}

impl Players {
    fn iter(&self) -> Values<'_, Dir, Arc<dyn Player>> {
        self.0.values()
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

/// A game consists of a set of players.
struct Game {
    players: Players,
    next_dealer: Dir,
    points: HashMap<Team, u8>,
}

impl Game {
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
        *points += outcome.points;
        Ok(outcome)
    }

    fn winner(&self) -> Option<Team> {
        for (team, points) in &self.points {
            if *points >= 10 {
                return Some(*team);
            }
        }
        None
    }
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
    Finish(Outcome),
}

#[derive(Debug)]
struct Outcome {
    team: Team,
    points: u8,
}

impl Round {
    pub fn new(dealer: Dir, mut deck: Deck) -> Self {
        let hands = dealer
            .next_n(4)
            .into_iter()
            .map(|dir| (dir, deck.take(5)))
            .collect();
        let top = deck.take(1)[0];
        Round::Deal(Deal { hands, dealer, top })
    }

    pub fn run(mut self, players: &Players) -> Result<Outcome, Error> {
        loop {
            self = self.next(players)?;
            if let Round::Finish(outcome) = self {
                return Ok(outcome);
            }
        }
    }

    fn next(self, players: &Players) -> Result<Self, Error> {
        match self {
            Round::Deal(deal) => {
                deal.deal(players);
                Ok(Round::BidTop(deal))
            }
            Round::BidTop(deal) => match deal.bid_top(players)? {
                Some(bid) => {
                    notify(players, Event::Bid(bid));
                    deal.dealer_pick_up_top(players, bid).map(Round::Play)
                }
                None => Ok(Round::BidOther(deal)),
            },
            Round::BidOther(deal) => {
                let bid = deal.bid_other(players)?;
                notify(players, Event::Bid(bid));
                Ok(Round::Play(deal.into_play(bid)))
            }
            Round::Play(mut play) => {
                let mut trick = play.lead_trick(players)?;
                play.follow_trick(players, &mut trick)?;
                let winner = trick.winner();
                notify(players, Event::Trick(winner, trick.clone()));
                play.collect_trick(winner, trick);
                Ok(match play.outcome() {
                    Some(outcome) => Round::Finish(outcome),
                    None => Round::Play(play),
                })
            }
            Round::Finish(_) => Ok(self),
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
    leader: Dir,
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
    fn lead_trick(&self) -> Card;

    /// Plays a card into an opened trick. The card must come from the player's
    /// hand. The player's card must follow the lead suit when possible.
    fn follow_trick(&self, trick: &Trick) -> Card;

    /// A notification of an event that all players can see.
    fn notify(&self, event: &Event);

    /// Indicates that the player has made an invalid play.
    ///
    /// The implementation may return true, if a retry is desired. Otherwise,
    /// the invalid play will be converted into a fatal error.
    fn invalid_play(&self, invalid: InvalidPlay) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub enum Contract {
    Partner,
    Alone,
}

#[derive(Debug, Clone, Copy)]
pub struct Bid {
    pub dir: Dir,
    pub suit: Suit,
    pub contract: Contract,
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

fn notify(players: &Players, event: Event) {
    for player in players.iter() {
        player.notify(&event)
    }
}

impl Deal {
    fn deal(&self, players: &Players) {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            let hand = self.hands.get(&dir).expect("hands");
            player.deal(self.dealer, hand.clone(), self.top);
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
            if discard(hand, card) {
                return Ok(self.into_play(bid));
            }
            // The dealer attempted to discard a card they do not hold.
            let invalid = InvalidPlay::CardNotHeld;
            if !dealer.invalid_play(invalid) {
                return Err(Error::InvalidPlay(self.dealer, invalid));
            }
        }
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
            leader: self.dealer.next(),
            bid,
        }
    }
}

impl Play {
    fn trump(&self) -> Suit {
        self.bid.suit
    }

    fn lead_trick(&mut self, players: &Players) -> Result<Trick, Error> {
        let hand = self.hands.get_mut(&self.leader).unwrap();
        let leader = &players[self.leader];
        loop {
            let card = leader.lead_trick();
            if discard(hand, card) {
                return Ok(Trick::new(self.trump(), self.leader, card));
            }
            let invalid = InvalidPlay::CardNotHeld;
            if !leader.invalid_play(invalid) {
                return Err(Error::InvalidPlay(self.leader, invalid));
            }
        }
    }

    fn follow_trick(&mut self, players: &Players, trick: &mut Trick) -> Result<(), Error> {
        for dir in self.leader.next_n(3) {
            let hand = self.hands.get_mut(&dir).unwrap();
            let player = &players[dir];
            loop {
                let card = player.follow_trick(trick);
                if !hand.contains(&card) {
                    let invalid = InvalidPlay::CardNotHeld;
                    if !player.invalid_play(invalid) {
                        return Err(Error::InvalidPlay(dir, invalid));
                    }
                } else if !trick.is_following_lead(hand, &card) {
                    let invalid = InvalidPlay::MustFollowLead;
                    if !player.invalid_play(invalid) {
                        return Err(Error::InvalidPlay(dir, invalid));
                    }
                } else {
                    discard(hand, card);
                    break;
                }
            }
        }
        Ok(())
    }

    fn collect_trick(&mut self, winner: Dir, trick: Trick) {
        self.leader = winner;
        self.tricks
            .get_mut(&winner)
            .expect("init tricks")
            .push(trick);
    }

    fn outcome(&self) -> Option<Outcome> {
        let mut total = 0;
        let mut bidder = 0;
        let mut defender = 0;
        let team = Team::from(self.bid.dir);
        for (dir, tricks) in &self.tricks {
            total += tricks.len();
            if Team::from(*dir) == team {
                bidder += tricks.len();
            } else {
                defender += tricks.len();
            }
        }
        if total == 5 {
            match (bidder, self.bid.contract) {
                (5, Contract::Alone) => Some(Outcome { team, points: 4 }),
                (5, Contract::Partner) => Some(Outcome { team, points: 2 }),
                (3 | 4, _) => Some(Outcome { team, points: 1 }),
                _ => Some(Outcome {
                    team: team.other(),
                    points: 2,
                }),
            }
        } else if defender > 2 {
            Some(Outcome {
                team: team.other(),
                points: 2,
            })
        } else {
            None
        }
    }
}

impl Trick {
    fn new(trump: Suit, leader: Dir, card: Card) -> Self {
        Self {
            trump,
            cards: vec![(leader, card)],
        }
    }

    pub fn leader(&self) -> Dir {
        self.cards[0].0
    }

    pub fn lead_card(&self) -> Card {
        self.cards[0].1
    }

    pub fn winner(&self) -> Dir {
        self.cards
            .iter()
            .max_by_key(|(_, card)| card.value(self.trump, self.lead_card()))
            .expect("non-empty")
            .0
    }

    /// Validate that the player is following the lead suit where possible.
    pub fn is_following_lead(&self, hand: &[Card], card: &Card) -> bool {
        let lead = self.lead_card();
        card.is_following(self.trump, lead)
            || !hand.iter().any(|c| c.is_following(self.trump, lead))
    }

    fn play(&mut self, dir: Dir, card: Card) {
        self.cards.push((dir, card));
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
        assert_eq!(trick('♠', &["N9♥"]).winner(), Dir::North);
        assert_eq!(
            trick('♠', &["N9♥", "ET♥", "SJ♥", "WQ♥"]).winner(),
            Dir::West
        );
        assert_eq!(
            trick('♠', &["NJ♠", "EK♠", "SA♣", "WJ♣"]).winner(),
            Dir::North
        );
        assert_eq!(
            trick('♠', &["NQ♠", "EK♠", "SA♣", "WJ♣"]).winner(),
            Dir::West
        );
    }
}
