//! Robot player
//!

use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, HashSet,
    },
    sync::{Arc, Mutex},
};

use itertools::Itertools;

use super::{Card, Contract, Dir, Event, InvalidPlay, Player, Suit, Trick};
use crate::game::euchre::{Rank, Team};

#[derive(Debug)]
struct Round {
    // Round state
    dealer: Dir,
    dealer_team: Team,
    top: Card,
    cards: Vec<Card>,
    contract: Option<Contract>,
}

#[derive(Debug)]
struct Inner {
    // Immutable state
    dir: Dir,
    team: Team,

    // Round state
    round: Option<Round>,

    // Game state
    our_score: u8,
    their_score: u8,
}

#[derive(Debug)]
pub struct Robot(Mutex<Inner>);

impl Player for Robot {
    fn deal(&self, dealer: Dir, cards: Vec<Card>, top: Card) {
        let mut inner = self.0.lock().unwrap();
        inner.deal(dealer, cards, top);
    }

    fn bid_top(&self) -> Option<bool> {
        let inner = self.0.lock().unwrap();
        inner.bid_top()
    }

    fn bid_other(&self) -> Option<(Suit, bool)> {
        let inner = self.0.lock().unwrap();
        inner.bid_other()
    }

    fn pick_up_top(&self, top: Card) -> Card {
        let mut inner = self.0.lock().unwrap();
        inner.pick_up_top(top)
    }

    fn lead_trick(&self) -> Card {
        let mut inner = self.0.lock().unwrap();
        inner.lead_trick()
    }

    fn follow_trick(&self, trick: &Trick) -> Card {
        let mut inner = self.0.lock().unwrap();
        inner.follow_trick(trick)
    }

    fn notify(&self, event: &Event) {
        let mut inner = self.0.lock().unwrap();
        inner.notify(event);
    }

    fn invalid_play(&self, _: InvalidPlay) -> bool {
        false
    }
}

impl Robot {
    pub fn new(dir: Dir) -> Self {
        Robot(Mutex::new(Inner::new(dir)))
    }

    pub fn as_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }
}

// Builds a map from non-trump suits to a set of card positions.
fn suit_map(cards: &[Card], trump: Suit) -> HashMap<Suit, Vec<usize>> {
    let mut suits: HashMap<_, Vec<usize>> = HashMap::new();
    for (idx, card) in cards.iter().enumerate() {
        if !card.is_trump(trump) {
            suits.entry(card.suit).or_default().push(idx)
        }
    }
    suits
}

impl Inner {
    fn new(dir: Dir) -> Self {
        Self {
            dir,
            team: Team::from(dir),
            round: None,
            our_score: 0,
            their_score: 0,
        }
    }
    fn deal(&mut self, dealer: Dir, cards: Vec<Card>, top: Card) {
        println!(
            "{:?}: {}",
            self.dir,
            cards.iter().map(|c| c.to_string()).join(", ")
        );
        self.round.replace(Round::new(dealer, cards, top));
    }

    fn bid_top(&self) -> Option<bool> {
        let round = self.round.as_ref().expect("no deal?");
        let score = self.z_score(round.top.suit, Some(round.top));
        println!("{:?}: z-score {}", self.dir, score);
        if score >= 9 {
            Some(false)
        } else {
            None
        }
    }

    fn bid_other(&self) -> Option<(Suit, bool)> {
        let round = self.round.as_ref().expect("no deal?");
        let mut best = (0, Suit::Club);
        for &suit in Suit::all_suits() {
            if suit != round.top.suit {
                let score = self.z_score(suit, None);
                if score >= 9 {
                    return Some((suit, false));
                } else if score > best.0 {
                    best = (score, suit);
                }
            }
        }
        if self.dir == round.dealer {
            Some((best.1, false))
        } else {
            None
        }
    }

    // An arbitrary rubrik based on Eric Zalas's "z-score".
    fn z_score(&self, trump: Suit, top: Option<Card>) -> u8 {
        let mut suits: HashSet<Suit> = HashSet::new();
        let mut score = 0;

        fn card_score(card: Card, trump: Suit) -> u8 {
            match (card.is_trump(trump), card.rank) {
                (true, Rank::Jack) => 3,
                (true, _) => 2,
                (false, Rank::Ace) => 1,
                _ => 0,
            }
        }

        // Intrinsic card values.
        let round = self.round.as_ref().expect("no deal?");
        for card in &round.cards {
            suits.insert(card.suit);
            score += card_score(*card, trump);
        }

        // Voids.
        score += match suits.len() {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => 0,
        };

        // Top card.
        if let Some(top) = top {
            let delta = card_score(top, trump);
            if round.dealer_team == self.team {
                score += delta;
            } else {
                score = score.saturating_sub(delta);
            }
        }

        score
    }

    fn pick_up_top(&mut self, top: Card) -> Card {
        let round = self.round.as_mut().expect("no deal?");
        round.cards.push(top);

        // Void a suit, if possible.
        let mut suits: HashMap<Suit, Option<usize>> = HashMap::new();
        for (idx, card) in round.cards.iter().enumerate() {
            match suits.entry(card.suit) {
                Occupied(mut e) => _ = e.get_mut().take(),
                Vacant(e) => _ = e.insert(Some(idx)),
            };
        }
        for (_, idx) in suits {
            if let Some(idx) = idx {
                return round.cards.remove(idx);
            }
        }

        // Can't void a suit, just remove the weakest card.
        round.cards.sort_unstable_by_key(|c| c.value(top.suit, *c));
        round.cards.remove(0)
    }

    fn lead_trick(&mut self) -> Card {
        let round = self.round.as_mut().expect("no deal?");
        if round.cards.len() == 1 {
            return round.cards.remove(0);
        }

        // First trick, defending
        //  - Singleton ace
        //  - Ace with one other card.
        //  - Least non-trump card.
        //
        // First trick, maker
        //  - Right bower, if in my hand
        //  - Least trump
        //
        // First trick, partner
        //  - Right bower, if in my hand
        //  - Least trump
        //  - Singleton ace
        //  - Ace with one other card
        //  - Least card

        let contract = round.contract.unwrap();
        let trump = contract.suit;
        if Team::from(contract.maker) == self.team {
            // Right bower
            let right = Card::new(Rank::Jack, trump);
            if let Some(idx) = round.cards.iter().position(|c| *c == right) {
                return round.cards.remove(idx);
            }

            round.cards.sort_unstable_by_key(|c| c.value(trump, *c));

            // Least trump on the first round, best trump on subsequent.
            if round.cards.len() == 5 {
                if let Some(idx) = round.cards.iter().position(|c| c.is_trump(trump)) {
                    return round.cards.remove(idx);
                }
            } else if let Some(idx) = round.cards.iter().rposition(|c| c.is_trump(trump)) {
                return round.cards.remove(idx);
            }
        }

        // Singleton ace, or ace with one other card.
        let suit_map = suit_map(&round.cards, trump);
        for threshold in [1, 2] {
            for (idx, card) in round.cards.iter().enumerate() {
                if !card.is_trump(trump)
                    && card.rank == Rank::Ace
                    && suit_map
                        .get(&card.suit)
                        .is_some_and(|v| v.len() == threshold)
                {
                    return round.cards.remove(idx);
                }
            }
        }

        round.cards.sort_unstable_by_key(|c| c.value(trump, *c));
        if round.cards.len() >= 4 {
            // Least card
            round.cards.remove(0)
        } else {
            // Best card
            round.cards.pop().unwrap()
        }
    }

    fn follow_trick(&mut self, trick: &Trick) -> Card {
        // Filter down to what cards I _can_ play.
        let round = self.round.as_mut().expect("no deal?");
        let cards = trick.filter(&round.cards);

        // The easiest choice is no choice at all.
        assert!(!cards.is_empty());
        if cards.len() == 1 {
            let card = cards[0];
            round.cards.retain(|c| *c != card);
            return card;
        }

        // Considerations:
        //  - Which position am I?
        //  - Is my partner already winning the trick?
        //  - Do I want to win the trick?
        //  - Parition cards into winning/losing.
        //  - Which cards do I want to get rid of?
        //
        //let position = trick.cards.len();
        //let partner_winning = trick.best().0 == self.dir.opposite();

        let (mut losing, mut winning): (Vec<_>, Vec<_>) = cards
            .into_iter()
            .partition(|c| c.value(trick.trump, trick.lead().1) < trick.best_value);

        let card = winning.pop().or_else(|| losing.pop()).unwrap();
        round.cards.retain(|c| *c != card);
        card
    }

    fn notify(&mut self, event: &Event) {
        let round = self.round.as_mut().expect("no deal?");
        match event {
            Event::Bid(contract) => round.contract = Some(*contract),
            Event::Trick(_) => (),
            Event::Round(outcome) => {
                if outcome.team == self.team {
                    self.our_score += outcome.points;
                } else {
                    self.their_score += outcome.points;
                }
            }
        }
    }
}

impl Round {
    fn new(dealer: Dir, cards: Vec<Card>, top: Card) -> Self {
        Self {
            dealer,
            dealer_team: Team::from(dealer),
            top,
            cards,
            contract: None,
        }
    }
}
