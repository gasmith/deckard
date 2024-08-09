//! Console interactive player.

use std::{fmt::Display, io::Write, str::FromStr, sync::Arc};

use ansi_term::{ANSIString, ANSIStrings};
use itertools::Itertools;

use super::{Card, Dir, Event, InvalidPlay, Player, Suit, Trick};

pub struct Console {
    dir: Dir,
    color: bool,
}

fn prompt<T: FromStr, S: Display>(prompt: S) -> T {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        let mut buffer = String::new();
        print!("{}", prompt);
        stdout.flush().expect("flush");
        stdin.read_line(&mut buffer).expect("read");
        let trimmed = buffer.trim();
        if !trimmed.is_empty() {
            if let Ok(obj) = T::from_str(trimmed) {
                return obj;
            }
            println!("Invalid input, try again");
        }
    }
}

impl Console {
    pub fn new(dir: Dir) -> Self {
        Self { dir, color: true }
    }

    pub fn into_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }

    fn format(&self, s: &ANSIStrings) -> String {
        if self.color {
            s.to_string()
        } else {
            ansi_term::unstyle(s)
        }
    }

    fn format_card(&self, card: Card) -> String {
        self.format(&ANSIStrings(&[card.to_ansi_string()]))
    }

    fn format_suit(&self, suit: Suit) -> String {
        self.format(&ANSIStrings(&[suit.to_ansi_string()]))
    }

    fn format_cards(&self, cards: &[Card]) -> String {
        let mut parts: Vec<ANSIString> = vec![];
        for (ii, card) in cards
            .iter()
            .sorted_unstable_by_key(|c| (c.suit, c.rank))
            .enumerate()
        {
            if ii > 0 {
                parts.push(", ".into());
            }
            parts.push(card.to_ansi_string());
        }
        self.format(&ANSIStrings(&parts))
    }

    fn format_trick(&self, trick: &Trick) -> String {
        let mut parts: Vec<ANSIString> = vec!["[".into()];
        for (i, (dir, card)) in trick.cards.iter().enumerate() {
            if i != 0 {
                parts.push(", ".into());
            }
            parts.push(format!("{dir:?}:").into());
            parts.push(card.to_ansi_string());
        }
        parts.push("]".into());
        self.format(&ANSIStrings(&parts))
    }
}

impl Player for Console {
    fn deal(&self, dealer: Dir, cards: Vec<Card>, top: Card) {
        println!("Dealer: {dealer:?}");
        println!("Top: {}", self.format_card(top));
        println!("{:?}: {}", self.dir, self.format_cards(&cards));
    }

    fn bid_top(&self) -> Option<bool> {
        if prompt::<bool, _>("Bid top? ") {
            let alone = prompt::<bool, _>("Alone? ");
            Some(alone)
        } else {
            None
        }
    }

    fn bid_other(&self) -> Option<(Suit, bool)> {
        if prompt::<bool, _>("Bid other? ") {
            let suit = prompt::<Suit, _>("Suit? ");
            let alone = prompt::<bool, _>("Alone? ");
            Some((suit, alone))
        } else {
            None
        }
    }

    fn pick_up_top(&self, _: Card) -> Card {
        prompt("Discard? ")
    }

    fn lead_trick(&self) -> Card {
        prompt("Lead? ")
    }

    fn follow_trick(&self, trick: &Trick) -> Card {
        println!("Trick: {}", self.format_trick(trick));
        prompt("Follow? ")
    }

    fn notify(&self, event: &Event) {
        match event {
            Event::Bid(contract) => {
                println!(
                    "{:?}: Bid {}{}",
                    contract.maker,
                    self.format_suit(contract.suit),
                    if contract.alone { " alone" } else { "" }
                );
            }
            Event::Trick(trick) => {
                println!(
                    "Trick: {} -> {:?}",
                    self.format_trick(trick),
                    trick.best().0
                );
            }
            Event::Round(outcome) => {
                println!("{:?}: Scores {}", outcome.team, outcome.points);
            }
        }
    }

    fn invalid_play(&self, err: InvalidPlay) -> bool {
        println!("Invalid play: {err:?}");
        true
    }
}
