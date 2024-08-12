//! Robot player

use std::collections::HashMap;
use std::sync::Arc;

use super::{ActionData, ActionType, Card, Player, PlayerState, Suit};
use crate::game::euchre::{Rank, Team};

const MIN_Z_SCORE: u8 = 8;
const MIN_LONER_Z_SCORE: u8 = 11;

#[derive(Debug, Clone)]
struct Hand {
    cards: Vec<Card>,
    trump: Suit,
    by_suit: HashMap<Suit, Vec<Card>>,
}

#[derive(Debug, Default)]
pub struct Robot {}

impl Player for Robot {
    fn take_action(&self, state: PlayerState, action: ActionType) -> ActionData {
        match action {
            ActionType::BidTop => bid_top(state),
            ActionType::BidOther => bid_other(state),
            ActionType::DealerDiscard => dealer_discard(state),
            ActionType::Lead => lead_trick(state),
            ActionType::Follow => follow_trick(state),
        }
    }
}

impl Robot {
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

fn bid_top(state: PlayerState) -> ActionData {
    let hand = Hand::new(state.hand.clone(), state.top.suit);
    let mut score = if state.seat.team() == state.dealer.team() {
        let mut alt_hand = hand.clone();
        alt_hand.push(state.top);
        if state.dealer == state.seat {
            // Dealer knows what to discard (e.g., for voids).
            alt_hand.dealer_discard();
        }
        alt_hand.z_score(None)
    } else {
        hand.z_score(Some(state.top))
    };
    if score >= MIN_Z_SCORE {
        if state.seat == state.dealer.opposite() {
            // If we're considering going alone, and the dealer is
            // opposite, ignore the top card. This could be more nuanced -
            // removing a trump from the game is worth _something_, but
            // it's probably not as good as having it your team's hands.
            // Hence the +1.
            score = hand.z_score(None) + 1;
        }
        ActionData::BidTop {
            alone: score >= MIN_LONER_Z_SCORE,
        }
    } else if score + 2 >= MIN_Z_SCORE
        && state.seat == state.dealer
        && Suit::all_suits()
            .iter()
            .filter(|&&s| s != state.top.suit)
            .all(|s| score > Hand::new(state.hand.clone(), *s).z_score(None))
    {
        //println!("{:?}: Better than getting stuck...", self.seat);
        ActionData::BidTop { alone: false }
    } else {
        ActionData::Pass
    }
}

fn bid_other(state: PlayerState) -> ActionData {
    let mut best = (0, Suit::Club);
    for &suit in Suit::all_suits() {
        if suit != state.top.suit {
            let score = Hand::new(state.hand.clone(), suit).z_score(None);
            //println!("{:?}: z-score for {} is {}", self.seat, suit, score);
            if score >= MIN_Z_SCORE {
                return ActionData::BidOther {
                    suit: best.1,
                    alone: score >= MIN_LONER_Z_SCORE,
                };
            } else if score > best.0 {
                best = (score, suit);
            }
        }
    }
    if state.seat == state.dealer {
        ActionData::BidOther {
            suit: best.1,
            alone: false,
        }
    } else {
        ActionData::Pass
    }
}

fn dealer_discard(state: PlayerState) -> ActionData {
    let contract = state.contract.expect("contract must be set");
    let mut hand = Hand::new(state.hand.clone(), contract.suit);
    let card = hand.dealer_discard();
    ActionData::Card { card }
}

fn lead_trick(state: PlayerState) -> ActionData {
    if state.hand.len() == 1 {
        // The easiest choice is no choice at all.
        return ActionData::Card {
            card: state.hand[0],
        };
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

    let contract = state.contract.expect("contract must be set");
    let mut hand = Hand::new(state.hand.clone(), contract.suit);
    let team = state.seat.team();
    let trump = contract.suit;
    if Team::from(contract.maker) == team {
        // Right bower
        let right = Card::new(Rank::Jack, trump);
        if let Some(card) = hand.discard(right) {
            return ActionData::Card { card };
        }

        hand.sort();

        if hand.len() == 5 {
            if let Some(&card) = hand.iter().find(|c| c.is_trump(trump)) {
                // Least trump on the first round
                return ActionData::Card { card };
            }
        } else if let Some(&card) = hand.iter().rev().find(|c| c.is_trump(trump)) {
            // Best trump on subsequent rounds
            return ActionData::Card { card };
        }
    }

    // Singleton ace, or ace with one other card.
    for threshold in [1, 2] {
        if let Some(&card) = hand
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
            return ActionData::Card { card };
        }
    }

    hand.sort();
    let card = if hand.len() >= 4 {
        // Least card
        hand.cards[0]
    } else if Team::from(contract.maker) != team {
        // Best non-trump card as defender
        if let Some(card) = hand.iter().rev().find(|c| !c.is_trump(trump)) {
            *card
        } else {
            *hand.cards.last().expect("non-empty")
        }
    } else {
        // Best card
        *hand.cards.last().expect("non-empty")
    };
    ActionData::Card { card }
}

fn follow_trick(state: PlayerState) -> ActionData {
    // Filter down to what cards I _can_ play.
    let trick = state.current_trick().expect("trick must be started");
    let cards = trick.filter(state.hand);
    if cards.len() == 1 {
        // The easiest choice is no choice at all.
        return ActionData::Card { card: cards[0] };
    }

    let contract = state.contract.expect("contract must be set");
    let trump = contract.suit;

    // Considerations:
    //  - Which position am I?
    //  - Is my partner already winning the trick?
    //  - Do I want to win the trick?
    //  - Parition cards into winning/losing.
    //  - Which cards do I want to get rid of?
    //  - When discarding, can I void a suit?
    //
    let position = trick.cards.len();
    let partner_winning = trick.best().0 == state.seat.opposite();

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
    } else if state.hand.len() >= 4
        && partner_winning
        && trick
            .get_card(state.seat.opposite())
            .is_some_and(|c| c.rank == Rank::Ace && !c.is_trump(trump))
    {
        // Trust our partner, if they play a non-trump Ace early on.
        least_valuable(losing, trump)
    } else {
        // Win with the most-valued card.
        most_valuable(winning, trump)
    };
    ActionData::Card { card }
}

fn discard(cards: &mut Vec<Card>, card: Card) -> Option<Card> {
    cards
        .iter()
        .position(|c| *c == card)
        .map(|idx| cards.remove(idx))
}

fn group_cards_by_suit(cards: &[Card], trump: Suit) -> HashMap<Suit, Vec<Card>> {
    let mut group: HashMap<_, Vec<_>> = HashMap::with_capacity(4);
    for card in cards {
        let suit = card.effective_suit(trump);
        group.entry(suit).or_default().push(*card)
    }
    group
}

impl Hand {
    pub fn new(cards: Vec<Card>, trump: Suit) -> Self {
        let by_suit = group_cards_by_suit(&cards, trump);
        Self {
            cards,
            trump,
            by_suit,
        }
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
        let suit = card.effective_suit(self.trump);
        self.cards.push(card);
        self.by_suit.entry(suit).or_default().push(card)
    }

    pub fn sort(&mut self) {
        let trump = self.trump;
        self.cards.sort_unstable_by_key(|c| c.value(trump, *c));
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

    pub fn dealer_discard(&mut self) -> Card {
        // Find a non-trump non-Ace that will void a suit.
        let voiding: Vec<_> = self
            .iter_by_suit()
            .filter_map(|(suit, cards)| {
                if *suit != self.trump && cards.len() == 1 && cards[0].rank != Rank::Ace {
                    Some(cards[0])
                } else {
                    None
                }
            })
            .collect();
        if !voiding.is_empty() {
            let card = least_valuable(voiding, self.trump);
            return self.must_discard(card);
        }

        // Find a non-trump non-Ace that will leave only an Ace behind.
        let near_voiding: Vec<_> = self
            .iter_by_suit()
            .filter_map(|(suit, cards)| {
                if *suit != self.trump && cards.len() == 2 {
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
            let card = least_valuable(near_voiding, self.trump);
            return self.must_discard(card);
        }

        // Can't void a suit, just remove the weakest card.
        self.sort();
        self.must_discard_first()
    }

    // A rubric based on Eric Zalas's "z-score".
    pub fn z_score(&self, opponent_top: Option<Card>) -> u8 {
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
            .fold(0, |acc, card| acc + card_score(*card, self.trump));

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
            .map(|card| card_score(card, self.trump))
            .unwrap_or(0);
        score.saturating_sub(penalty)
    }
}
