use std::cmp::Ordering;

use rand::prelude::*;

pub mod french;
use crate::french::{Card, Deck, Rank};

#[derive(Default)]
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
    fn draw<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Card {
        if self.draw.is_empty() {
            self.discard.shuffle(rng);
            std::mem::swap(&mut self.draw, &mut self.discard);
        }
        let Some(card) = self.draw.pop() else {
            panic!("{} is out of cards", self.name);
        };
        card
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

fn main() {
    let mut rng = rand::thread_rng();
    let mut deck = Deck::standard().with_cards([Card::Joker, Card::Joker]);
    deck.shuffle(&mut rng);

    let mut a = Hand::new("a", deck.take(deck.len() / 2));
    let mut b = Hand::new("b", deck.take(deck.len()));
    for i in 0..1000 {
        println!("round {i} ({} {})", a.len(), b.len());
        let mut atop = a.draw(&mut rng);
        let mut btop = b.draw(&mut rng);
        let mut extra = vec![];
        loop {
            println!("  {atop} vs {btop}");
            match cmp(atop, btop) {
                Ordering::Greater => {
                    a.discard([atop, btop]);
                    a.discard(extra);
                    break;
                }
                Ordering::Less => {
                    b.discard([atop, btop]);
                    b.discard(extra);
                    break;
                }
                Ordering::Equal => {
                    extra.extend([
                        atop,
                        btop,
                        a.draw(&mut rng),
                        b.draw(&mut rng),
                        a.draw(&mut rng),
                        b.draw(&mut rng),
                    ]);
                    atop = a.draw(&mut rng);
                    btop = b.draw(&mut rng);
                }
            }
        }
    }
    println!("final standings");
    println!("a: {}", a.len());
    println!("b: {}", b.len());
}
