//! Tricks

use std::fmt::Display;

use super::{Card, Dir, Suit};

#[derive(Debug, Clone)]
pub struct Trick {
    pub trump: Suit,
    pub cards: Vec<(Dir, Card)>,
    pub winner: Dir,
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
            winner: leader,
        }
    }

    /// The leading player.
    pub fn leader(&self) -> Dir {
        self.cards[0].0
    }

    /// The leading card.
    pub fn lead(&self) -> Card {
        self.cards[0].1
    }

    /// Validate that the player is following the lead suit where possible.
    pub fn is_following_lead(&self, hand: &[Card], card: &Card) -> bool {
        let lead = self.lead();
        card.is_following(self.trump, lead)
            || !hand.iter().any(|c| c.is_following(self.trump, lead))
    }

    /// Plays a card into the trick.
    pub fn play(&mut self, dir: Dir, card: Card) {
        self.cards.push((dir, card));
        self.winner = self.calculate_winner();
    }

    /// The current winner of the trick.
    fn calculate_winner(&self) -> Dir {
        self.cards
            .iter()
            .max_by_key(|(_, card)| card.value(self.trump, self.lead()))
            .expect("non-empty")
            .0
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
    fn test_trick_winner() {
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
            assert_eq!(case.expect, case.trick.winner);
        }
    }
}
