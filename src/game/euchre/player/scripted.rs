//! A scripted player, for testing.

use std::str::FromStr;
use std::sync::Mutex;
use std::{collections::VecDeque, sync::Arc};

use super::{Card, Dir, Event, InvalidPlay, Player, Suit, Trick};

#[derive(Debug, Default, Clone)]
struct Inner {
    bids_top: Option<bool>,
    bids_other: Option<(Suit, bool)>,
    discards: Option<Card>,
    leads: VecDeque<Card>,
    follows: VecDeque<Card>,
}

#[derive(Debug, Default)]
pub struct ScriptedPlayer(Mutex<Inner>);

impl Player for ScriptedPlayer {
    fn deal(&self, _: Dir, _: Vec<Card>, _: Card) {}

    fn bid_top(&self) -> Option<bool> {
        let inner = self.0.lock().unwrap();
        inner.bids_top
    }

    fn bid_other(&self) -> Option<(Suit, bool)> {
        let inner = self.0.lock().unwrap();
        inner.bids_other
    }

    fn pick_up_top(&self, _: Card) -> Card {
        let inner = self.0.lock().unwrap();
        inner.discards.unwrap()
    }

    fn lead_trick(&self) -> Card {
        let mut inner = self.0.lock().unwrap();
        inner.leads.pop_front().unwrap()
    }

    fn follow_trick(&self, _: &Trick) -> Card {
        let mut inner = self.0.lock().unwrap();
        inner.follows.pop_front().unwrap()
    }

    fn notify(&self, _: &Event) {}

    fn invalid_play(&self, _: InvalidPlay) -> bool {
        false
    }
}

impl ScriptedPlayer {
    pub fn into_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }

    pub fn bids_top(self, alone: bool) -> Self {
        let mut inner = self.0.lock().unwrap();
        inner.bids_top.replace(alone);
        drop(inner);
        self
    }

    pub fn bids_other(self, suit: Suit, alone: bool) -> Self {
        let mut inner = self.0.lock().unwrap();
        inner.bids_other.replace((suit, alone));
        drop(inner);
        self
    }

    pub fn discards(self, card: &str) -> Self {
        let card = Card::from_str(card).unwrap();
        let mut inner = self.0.lock().unwrap();
        inner.discards.replace(card);
        drop(inner);
        self
    }

    pub fn leads(self, card: &str) -> Self {
        let card = Card::from_str(card).unwrap();
        let mut inner = self.0.lock().unwrap();
        inner.leads.push_back(card);
        drop(inner);
        self
    }

    pub fn follows(self, card: &str) -> Self {
        let card = Card::from_str(card).unwrap();
        let mut inner = self.0.lock().unwrap();
        inner.follows.push_back(card);
        drop(inner);
        self
    }
}
