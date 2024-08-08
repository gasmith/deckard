//! Tricks

use std::fmt::Display;

use super::{Card, Dir, Suit};

#[derive(Debug, Clone)]
pub struct Trick {
    pub trump: Suit,
    pub cards: Vec<(Dir, Card)>,
    pub best: usize,
    pub best_value: u8,
}

impl Display for Trick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, (dir, card)) in self.cards.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{dir:?}:{card}")?;
        }
        write!(f, "]")
    }
}

impl Trick {
    /// Opens a new trick.
    pub fn new(trump: Suit, leader: Dir, card: Card) -> Self {
        Self {
            trump,
            cards: vec![(leader, card)],
            best: 0,
            best_value: card.value(trump, card),
        }
    }

    /// The lead card.
    pub fn lead(&self) -> (Dir, Card) {
        self.cards[0]
    }

    /// The best card.
    pub fn best(&self) -> (Dir, Card) {
        self.cards[self.best]
    }

    /// Validate that the player is following the lead suit where possible.
    pub fn is_following_lead(&self, hand: &[Card], card: &Card) -> bool {
        let lead_card = self.lead().1;
        card.is_following(self.trump, lead_card)
            || !hand.iter().any(|c| c.is_following(self.trump, lead_card))
    }

    /// Filters the hand down to the set of playable cards.
    pub fn filter(&self, hand: &[Card]) -> Vec<Card> {
        let following: Vec<_> = hand
            .iter()
            .filter(|c| c.is_following(self.trump, self.lead().1))
            .copied()
            .collect();
        if following.is_empty() {
            hand.to_vec()
        } else {
            following
        }
    }

    /// Plays a card into the trick.
    pub fn play(&mut self, dir: Dir, card: Card) {
        let card_value = card.value(self.trump, self.lead().1);
        if card_value > self.best_value {
            self.best_value = card_value;
            self.best = self.cards.len();
        }
        self.cards.push((dir, card));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::euchre::Rank;

    fn trick(trump: char, cards: &[&str]) -> Trick {
        let trump = Suit::from_char(trump).unwrap();
        let mut cards = cards.iter().map(|s| {
            let mut chars = s.chars();
            let dir = chars.next().and_then(Dir::from_char).unwrap();
            let rank = chars.next().and_then(Rank::from_char).unwrap();
            let suit = chars.next().and_then(Suit::from_char).unwrap();
            assert!(chars.next().is_none());
            (dir, Card { rank, suit })
        });
        let (dir, card) = cards.next().unwrap();
        let mut trick = Trick::new(trump, dir, card);
        for (dir, card) in cards {
            trick.play(dir, card);
        }
        trick
    }

    #[test]
    fn test_trick_best() {
        struct Case {
            trick: Trick,
            expect: Dir,
        }

        fn case(cards: &[&str], expect: char) -> Case {
            Case {
                trick: trick('H', cards),
                expect: Dir::from_char(expect).unwrap(),
            }
        }

        let cases = [
            case(&["N9S"], 'N'),
            case(&["N9S", "ETS"], 'E'),
            case(&["ETS", "N9S"], 'E'),
            case(&["NTS", "E9S"], 'N'),
            case(&["NJS", "ETS"], 'N'),
            case(&["NQS", "EJS"], 'N'),
            case(&["NKS", "EQS"], 'N'),
            case(&["NAS", "EKS"], 'N'),
            case(&["NAS", "EKS"], 'N'),
            case(&["NAS", "E9H"], 'E'),
            case(&["NAS", "EJD"], 'E'),
            case(&["NAS", "EJH"], 'E'),
            case(&["NJH", "EJD"], 'N'),
            case(&["NJD", "EJH"], 'E'),
        ];
        for case in cases {
            println!("{} -> {:?}", &case.trick, &case.expect);
            assert_eq!(case.expect, case.trick.best().0);
        }
    }
}
