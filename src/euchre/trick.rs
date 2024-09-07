//! Trick

use std::fmt::Display;

use crate::euchre::{Card, Seat, Suit};

/// A trick played during a round.
#[derive(Debug, Clone)]
pub struct Trick {
    /// The trump suit for this trick.
    pub trump: Suit,
    /// The cards that have been played into this trick.
    pub cards: Vec<(Seat, Card)>,
    /// The index of the best card played.
    pub best: usize,
    /// The value of the best card played.
    pub best_value: u8,
}

impl Display for Trick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, (seat, card)) in self.cards.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{seat:?}:{card}")?;
        }
        write!(f, "]")
    }
}

impl Trick {
    /// Creates a new trick.
    pub fn new(trump: Suit, leader: Seat, card: Card) -> Self {
        Self {
            trump,
            cards: vec![(leader, card)],
            best: 0,
            best_value: card.value(trump, card),
        }
    }

    /// The number of cards played into this trick.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// The lead card.
    pub fn lead(&self) -> (Seat, Card) {
        self.cards[0]
    }

    /// The best card.
    pub fn best(&self) -> (Seat, Card) {
        self.cards[self.best]
    }

    /// Return the specified player's card in this trick.
    pub fn get_card(&self, seat: Seat) -> Option<Card> {
        self.cards
            .iter()
            .find_map(|(s, c)| if *s == seat { Some(*c) } else { None })
    }

    /// Validate that the player is following the lead suit where possible.
    pub fn is_following_lead(&self, hand: &[Card], card: Card) -> bool {
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
    pub fn play(&mut self, seat: Seat, card: Card) {
        let card_value = card.value(self.trump, self.lead().1);
        if card_value > self.best_value {
            self.best_value = card_value;
            self.best = self.cards.len();
        }
        self.cards.push((seat, card));
    }
}

#[cfg(test)]
mod test {
    use std::convert::{TryFrom, TryInto};

    use super::*;

    fn trick(trump: char, cards: &[&str]) -> Trick {
        let trump = Suit::try_from(trump).unwrap();
        let mut cards = cards.iter().map(|s| {
            let mut chars = s.chars();
            let seat = chars.next().unwrap().try_into().unwrap();
            let rank = chars.next().unwrap().try_into().unwrap();
            let suit = chars.next().unwrap().try_into().unwrap();
            assert!(chars.next().is_none());
            (seat, Card { rank, suit })
        });
        let (seat, card) = cards.next().unwrap();
        let mut trick = Trick::new(trump, seat, card);
        for (seat, card) in cards {
            trick.play(seat, card);
        }
        trick
    }

    #[test]
    fn test_trick_best() {
        struct Case {
            trick: Trick,
            expect: Seat,
        }

        fn case(cards: &[&str], expect: char) -> Case {
            Case {
                trick: trick('H', cards),
                expect: expect.try_into().unwrap(),
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
