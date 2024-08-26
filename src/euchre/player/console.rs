//! Simple console interactive player.

use std::{fmt::Display, io::Write, str::FromStr, sync::Arc};

use ansi_term::{ANSIString, ANSIStrings};
use itertools::Itertools;

use super::{ActionData, ActionType, Card, Event, Player, PlayerError, PlayerState, Suit, Trick};

pub struct Console {
    color: bool,
}
impl Default for Console {
    fn default() -> Self {
        Self::new(true)
    }
}

fn prompt<T: FromStr, S: Display>(prompt: S) -> T {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        let mut buffer = String::new();
        print!("{prompt}");
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
    pub fn new(color: bool) -> Self {
        Self { color }
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
        for (i, (seat, card)) in trick.cards.iter().enumerate() {
            if i != 0 {
                parts.push(", ".into());
            }
            parts.push(format!("{seat:?}:").into());
            parts.push(card.to_ansi_string());
        }
        parts.push("]".into());
        self.format(&ANSIStrings(&parts))
    }

    fn bid_top(&self, state: &PlayerState) -> ActionData {
        println!("Hand: {}", self.format_cards(state.hand));
        if prompt::<bool, _>("Bid top? ") {
            let alone = prompt::<bool, _>("Alone? ");
            ActionData::Call {
                suit: state.top.suit,
                alone,
            }
        } else {
            ActionData::Pass
        }
    }

    #[allow(clippy::unused_self)]
    fn bid_other(&self, _: &PlayerState) -> ActionData {
        if prompt::<bool, _>("Bid other? ") {
            let suit = prompt::<Suit, _>("Suit? ");
            let alone = prompt::<bool, _>("Alone? ");
            ActionData::Call { suit, alone }
        } else {
            ActionData::Pass
        }
    }

    fn dealer_discard(&self, state: &PlayerState) -> ActionData {
        println!("Hand: {}", self.format_cards(state.hand));
        let card = prompt("Discard? ");
        ActionData::Card { card }
    }

    fn lead(&self, state: &PlayerState) -> ActionData {
        println!("Hand: {}", self.format_cards(state.hand));
        let card = prompt("Lead? ");
        ActionData::Card { card }
    }

    fn follow(&self, state: &PlayerState) -> ActionData {
        let trick = state.tricks.last().unwrap();
        println!("Trick: {}", self.format_trick(trick));
        println!("Hand: {}", self.format_cards(state.hand));
        let card = prompt("Follow? ");
        ActionData::Card { card }
    }
}

impl Player for Console {
    fn take_action(&self, state: PlayerState, action: ActionType) -> ActionData {
        match action {
            ActionType::BidTop => self.bid_top(&state),
            ActionType::BidOther => self.bid_other(&state),
            ActionType::DealerDiscard => self.dealer_discard(&state),
            ActionType::Lead => self.lead(&state),
            ActionType::Follow => self.follow(&state),
        }
    }

    fn notify(&self, _: PlayerState, event: &Event) {
        match event {
            Event::Deal(dealer, top) => {
                println!("Dealer: {dealer}");
                println!("Top card: {}", self.format_card(*top));
            }
            Event::Call(contract) => {
                println!(
                    "{:?}: Called {}{}",
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
                println!("{:}: {} points", outcome.team, outcome.points);
            }
            Event::Game(outcome) => {
                println!("{:} wins!", outcome.team);
            }
        }
    }

    fn handle_error(&self, err: PlayerError) -> bool {
        println!("Error: {err:?}");
        true
    }
}
