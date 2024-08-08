//! Robot player
//!

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use itertools::Itertools;

use super::{Card, Contract, Dir, Event, InvalidPlay, Player, Suit, Trick};
use crate::game::euchre::{Rank, Team};

const MIN_Z_SCORE: u8 = 8;
const MIN_LONER_Z_SCORE: u8 = 11;

#[derive(Debug, Clone)]
struct Hand {
    cards: Vec<Card>,
    trump: Option<Suit>,
    by_suit: HashMap<Suit, Vec<Card>>,
}

/// State provided during the deal.
#[derive(Debug)]
struct Deal {
    dealer: Dir,
    dealer_team: Team,
    top: Card,
    hand: Hand,
}

#[derive(Debug)]
struct Inner {
    // Immutable state
    dir: Dir,
    team: Team,

    // Round state
    deal: Option<Deal>,
    contract: Option<Contract>,

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
        let card = inner.pick_up_top(top);
        println!("{:?}: Discard {card}", inner.dir);
        card
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

    pub fn into_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }
}

fn least_valuable(mut cards: Vec<Card>, trump: Suit) -> Card {
    cards.sort_unstable_by_key(|c| c.value(trump, *c));
    cards[0]
}

fn most_valuable(mut cards: Vec<Card>, trump: Suit) -> Card {
    cards.sort_unstable_by_key(|c| c.value(trump, *c));
    cards.pop().expect("non-empty")
}

impl Inner {
    fn new(dir: Dir) -> Self {
        Self {
            dir,
            team: Team::from(dir),
            deal: None,
            contract: None,
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
        self.deal.replace(Deal::new(dealer, cards, top));
        self.contract = None;
    }

    fn bid_top(&self) -> Option<bool> {
        let deal = self.deal.as_ref().expect("no deal?");
        let score = self.z_score(deal.top.suit, Some(deal.top));
        println!("{:?}: z-score for {} is {}", self.dir, deal.top.suit, score);
        if score >= MIN_Z_SCORE {
            Some(score >= MIN_LONER_Z_SCORE)
        } else if score + 2 >= MIN_Z_SCORE
            && deal.dealer == self.dir
            && Suit::all_suits()
                .iter()
                .all(|s| *s == deal.top.suit || score > self.z_score(*s, None))
        {
            println!("{:?}: Better than getting stuck...", self.dir);
            Some(false)
        } else {
            None
        }
    }

    fn bid_other(&self) -> Option<(Suit, bool)> {
        let deal = self.deal.as_ref().expect("no deal?");
        let mut best = (0, Suit::Club);
        for &suit in Suit::all_suits() {
            if suit != deal.top.suit {
                let score = self.z_score(suit, None);
                println!("{:?}: z-score for {} is {}", self.dir, suit, score);
                if score >= MIN_Z_SCORE {
                    return Some((suit, score >= MIN_LONER_Z_SCORE));
                } else if score > best.0 {
                    best = (score, suit);
                }
            }
        }
        if self.dir == deal.dealer {
            Some((best.1, false))
        } else {
            None
        }
    }

    fn z_score(&self, trump: Suit, top: Option<Card>) -> u8 {
        let deal = self.deal.as_ref().expect("no deal?");
        let mut hand = deal.hand.clone_with_trump(Some(trump));
        if top.is_some() && deal.dealer_team == self.team {
            // Top card goes to our team.
            hand.push(deal.top);
            if deal.dealer == self.dir {
                // Dealer knows what to discard (e.g., for voids).
                hand.dealer_discard();
            }
            hand.z_score(None)
        } else {
            // No top card, or it goes to opponent team.
            hand.z_score(top)
        }
    }

    fn notify(&mut self, event: &Event) {
        match event {
            Event::Bid(contract) => {
                let deal = self.deal.as_mut().expect("no deal?");
                deal.hand.set_trump(Some(contract.suit));
                self.contract = Some(*contract);
            }
            Event::Round(outcome) => {
                if outcome.team == self.team {
                    self.our_score += outcome.points;
                } else {
                    self.their_score += outcome.points;
                }
            }
            Event::Trick(_) => (),
        }
    }

    fn pick_up_top(&mut self, top: Card) -> Card {
        let trump = top.suit;
        let hand = self.deal.as_mut().map(|d| &mut d.hand).expect("no deal?");
        assert!(hand.trump().is_some_and(|s| s == trump));
        hand.push(top);
        hand.dealer_discard()
    }

    fn lead_trick(&mut self) -> Card {
        let deal = self.deal.as_mut().expect("no deal?");
        let hand = &mut deal.hand;
        if hand.len() == 1 {
            return hand.must_discard_first();
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

        let contract = self.contract.expect("no contract?");
        let trump = contract.suit;
        if Team::from(contract.maker) == self.team {
            // Right bower
            let right = Card::new(Rank::Jack, trump);
            if let Some(card) = hand.discard(right) {
                return card;
            }

            hand.sort();

            if hand.len() == 5 {
                if let Some(card) = hand.iter().find(|c| c.is_trump(trump)) {
                    // Least trump on the first round
                    return hand.must_discard(*card);
                }
            } else if let Some(card) = hand.iter().rev().find(|c| c.is_trump(trump)) {
                // Best trump on subsequent rounds
                return hand.must_discard(*card);
            }
        }

        // Singleton ace, or ace with one other card.
        for threshold in [1, 2] {
            if let Some(card) = hand
                .iter_by_suit()
                .filter_map(|(suit, cards)| {
                    if *suit != trump && cards.len() == threshold {
                        cards.iter().find(|card| card.rank == Rank::Ace)
                    } else {
                        None
                    }
                })
                .next()
            {
                return hand.must_discard(*card);
            }
        }

        hand.sort();
        if hand.len() >= 4 {
            // Least card
            hand.must_discard_first()
        } else {
            // Best card
            hand.must_discard_last()
        }
    }

    fn follow_trick(&mut self, trick: &Trick) -> Card {
        // Filter down to what cards I _can_ play.
        let deal = self.deal.as_mut().expect("no deal?");
        let trump = self.contract.map(|c| c.suit).expect("no contract?");
        let hand = &mut deal.hand;
        let cards = trick.filter(hand.cards());

        // The easiest choice is no choice at all.
        assert!(!cards.is_empty());
        if cards.len() == 1 {
            return hand.must_discard(cards[0]);
        }

        // Considerations:
        //  - Which position am I?
        //  - Is my partner already winning the trick?
        //  - Do I want to win the trick?
        //  - Parition cards into winning/losing.
        //  - Which cards do I want to get rid of?
        //  - When discarding, can I void a suit?
        //
        let position = trick.cards.len();
        let partner_winning = trick.best().0 == self.dir.opposite();

        let (losing, winning): (Vec<_>, Vec<_>) = cards
            .into_iter()
            .partition(|c| c.value(trick.trump, trick.lead().1) < trick.best_value);

        let card = if winning.is_empty() {
            // Always lose with the least-valued card.
            least_valuable(losing, trump)
        } else if position == 3 {
            // If playing last, we have a choice:
            if partner_winning && !losing.is_empty() {
                // If our partner is winning, let them win.
                least_valuable(losing, trump)
            } else {
                // Win with the least-valued card.
                least_valuable(winning, trump)
            }
        } else if hand.len() >= 4
            && partner_winning
            && trick
                .get_card(self.dir.opposite())
                .is_some_and(|c| c.rank == Rank::Ace && !c.is_trump(trump))
        {
            // Trust our partner, if they play a non-trump Ace early on.
            least_valuable(losing, trump)
        } else {
            // Win with the most-valued card.
            most_valuable(winning, trump)
        };
        hand.must_discard(card)
    }
}

impl Deal {
    fn new(dealer: Dir, cards: Vec<Card>, top: Card) -> Self {
        Self {
            dealer,
            dealer_team: Team::from(dealer),
            top,
            hand: Hand::new(cards, None),
        }
    }
}

fn discard(cards: &mut Vec<Card>, card: Card) -> Option<Card> {
    cards
        .iter()
        .position(|c| *c == card)
        .map(|idx| cards.remove(idx))
}

fn effective_suit(card: Card, trump: Option<Suit>) -> Suit {
    trump
        .map(|trump| card.effective_suit(trump))
        .unwrap_or(card.suit)
}

fn group_cards_by_suit(cards: &[Card], trump: Option<Suit>) -> HashMap<Suit, Vec<Card>> {
    let mut group: HashMap<_, Vec<_>> = HashMap::with_capacity(4);
    for card in cards {
        let suit = effective_suit(*card, trump);
        group.entry(suit).or_default().push(*card)
    }
    group
}

impl Hand {
    pub fn new(cards: Vec<Card>, trump: Option<Suit>) -> Self {
        let by_suit = group_cards_by_suit(&cards, trump);
        Self {
            cards,
            trump,
            by_suit,
        }
    }

    pub fn clone_with_trump(&self, trump: Option<Suit>) -> Self {
        if self.trump == trump {
            self.clone()
        } else {
            Self::new(self.cards.clone(), trump)
        }
    }

    pub fn set_trump(&mut self, trump: Option<Suit>) {
        self.trump = trump;
        self.by_suit = group_cards_by_suit(&self.cards, trump);
    }

    pub fn trump(&self) -> Option<Suit> {
        self.trump
    }

    pub fn cards(&self) -> &Vec<Card> {
        &self.cards
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Card> {
        self.cards.iter()
    }

    pub fn iter_by_suit(&self) -> std::collections::hash_map::Iter<'_, Suit, Vec<Card>> {
        self.by_suit.iter()
    }

    pub fn num_suits(&self) -> usize {
        self.by_suit.len()
    }

    pub fn push(&mut self, card: Card) {
        let suit = effective_suit(card, self.trump);
        self.cards.push(card);
        self.by_suit.entry(suit).or_default().push(card)
    }

    pub fn sort(&mut self) {
        let trump = self.trump;
        self.cards.sort_unstable_by_key(|c| match trump {
            Some(trump) => c.value(trump, *c),
            None => c.trumpless_value(*c),
        });
    }

    pub fn discard(&mut self, card: Card) -> Option<Card> {
        discard(&mut self.cards, card).map(|card| {
            self.by_suit = group_cards_by_suit(&self.cards, self.trump);
            card
        })
    }

    pub fn must_discard(&mut self, card: Card) -> Card {
        self.discard(card).expect("hand must contain card")
    }

    pub fn discard_first(&mut self) -> Option<Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn must_discard_first(&mut self) -> Card {
        self.discard_first().expect("hand is non-empty")
    }

    pub fn discard_last(&mut self) -> Option<Card> {
        if self.cards.is_empty() {
            None
        } else {
            self.cards.pop()
        }
    }

    pub fn must_discard_last(&mut self) -> Card {
        self.discard_last().expect("hand is non-empty")
    }

    pub fn dealer_discard(&mut self) -> Card {
        let trump = self.trump.expect("trump must be set");

        // Find a non-trump non-Ace that will void a suit.
        let voiding: Vec<_> = self
            .iter_by_suit()
            .filter_map(|(suit, cards)| {
                if *suit != trump && cards.len() == 1 && cards[0].rank != Rank::Ace {
                    Some(cards[0])
                } else {
                    None
                }
            })
            .collect();
        if !voiding.is_empty() {
            let card = least_valuable(voiding, trump);
            return self.must_discard(card);
        }

        // Find a non-trump non-Ace that will leave only an Ace behind.
        let near_voiding: Vec<_> = self
            .iter_by_suit()
            .filter_map(|(suit, cards)| {
                if *suit != trump && cards.len() == 2 {
                    match (cards[0].rank, cards[1].rank) {
                        (Rank::Ace, _) => Some(cards[1]),
                        (_, Rank::Ace) => Some(cards[0]),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();
        if !near_voiding.is_empty() {
            let card = least_valuable(near_voiding, trump);
            return self.must_discard(card);
        }

        // Can't void a suit, just remove the weakest card.
        self.sort();
        self.must_discard_first()
    }

    // A rubric based on Eric Zalas's "z-score".
    pub fn z_score(&self, opponent_top: Option<Card>) -> u8 {
        let trump = self.trump.expect("trump must be set");
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
        score += self
            .iter()
            .fold(0, |acc, card| acc + card_score(*card, trump));

        // Voids.
        score += match self.num_suits() {
            1 => 3,
            2 => 2,
            3 => 1,
            4 => 0,
            _ => unreachable!(),
        };

        // Top card given to opponent.
        let penalty = opponent_top
            .map(|card| card_score(card, trump))
            .unwrap_or(0);
        score.saturating_sub(penalty)
    }
}
