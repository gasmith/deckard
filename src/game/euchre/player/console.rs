//! Console interactive player.

use std::{fmt::Display, io::Write, str::FromStr, sync::Arc};

use itertools::Itertools;

use super::{Card, Dir, Event, InvalidPlay, Player, Suit, Trick};

pub struct Console {
    dir: Dir,
}

impl Console {
    pub fn new(dir: Dir) -> Self {
        Self { dir }
    }

    pub fn into_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }
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

impl Player for Console {
    fn deal(&self, _: Dir, cards: Vec<Card>, _: Card) {
        println!(
            "{:?}: {}",
            self.dir,
            cards.iter().map(|c| c.to_string()).join(", ")
        );
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
        println!("Trick so far: {trick}");
        prompt("Follow? ")
    }

    fn notify(&self, _: &Event) {}

    fn invalid_play(&self, err: InvalidPlay) -> bool {
        println!("Invalid play: {err:?}");
        true
    }
}
