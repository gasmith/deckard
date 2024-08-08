use std::cmp::Ordering;

use rand::prelude::*;

use crate::french::{Card, Deck, Rank};

pub enum DrawError {
    OutOfCards(String),
}

#[derive(Clone)]
pub struct War {
    h1: Hand,
    h2: Hand,
}

impl War {
    pub fn new(mut deck: Deck, p1: &str, p2: &str) -> Self {
        let h1 = Hand::new(p1, deck.take(deck.len() / 2));
        let h2 = Hand::new(p2, deck.take(deck.len()));
        assert_eq!(deck.len(), 0);
        Self { h1, h2 }
    }

    pub fn play_round<R: Rng>(&mut self, rng: &mut R) -> Result<(), DrawError> {
        let mut c1 = self.h1.draw(rng)?;
        let mut c2 = self.h2.draw(rng)?;
        let mut extra = vec![];
        loop {
            print!("{c1} vs {c2} ");
            match cmp(c1, c2) {
                Ordering::Greater => {
                    self.h1.discard([c1, c2]);
                    self.h1.discard(extra);
                    println!("-> {} ({})", self.h1.name, self.h1.len());
                    break;
                }
                Ordering::Less => {
                    self.h2.discard([c1, c2]);
                    self.h2.discard(extra);
                    println!("-> {} ({})", self.h2.name, self.h2.len());
                    break;
                }
                Ordering::Equal => {
                    print!("-> ");
                    extra.extend([c1, c2, self.h1.draw(rng)?, self.h2.draw(rng)?]);
                    c1 = self.h1.draw(rng)?;
                    c2 = self.h2.draw(rng)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Default, Clone)]
struct Hand {
    name: String,
    draw: Vec<Card>,
    discard: Vec<Card>,
}

impl Hand {
    fn new<I: IntoIterator<Item = Card>>(name: &str, cards: I) -> Self {
        Hand {
            name: name.to_string(),
            draw: cards.into_iter().collect(),
            discard: vec![],
        }
    }
    fn draw<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<Card, DrawError> {
        if self.draw.is_empty() {
            self.discard.shuffle(rng);
            std::mem::swap(&mut self.draw, &mut self.discard);
        }
        self.draw
            .pop()
            .ok_or_else(|| DrawError::OutOfCards(self.name.clone()))
    }
    fn discard<I: IntoIterator<Item = Card>>(&mut self, cards: I) {
        self.discard.extend(cards);
    }
    fn len(&self) -> usize {
        self.draw.len() + self.discard.len()
    }
}

fn rank_value(r: Rank) -> u8 {
    match r {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

fn cmp(a: Card, b: Card) -> Ordering {
    match (a, b) {
        (Card::Joker | Card::BigJoker, Card::Joker | Card::BigJoker) => Ordering::Equal,
        (Card::Joker | Card::BigJoker, _) => Ordering::Greater,
        (_, Card::Joker | Card::BigJoker) => Ordering::Less,
        (Card::RankSuit(ar, _), Card::RankSuit(br, _)) => rank_value(ar).cmp(&rank_value(br)),
    }
}

pub fn main() {
    let mut rng = rand::thread_rng();
    let mut deck = Deck::standard().with_cards([Card::Joker, Card::Joker]);
    deck.shuffle(&mut rng);
    let mut war = War::new(deck, "a", "b");
    for i in 1.. {
        print!("round {i}: ");
        if let Err(DrawError::OutOfCards(name)) = war.play_round(&mut rng) {
            println!("player {name} is out of cards");
            break;
        }
    }
}
