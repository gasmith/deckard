//! A scripted player, for testing.

use std::sync::Mutex;
use std::{collections::VecDeque, sync::Arc};

use super::{ActionData, ActionType, Player, PlayerState};

#[derive(Debug, Default)]
pub struct ScriptedPlayer(Mutex<VecDeque<ActionData>>);

impl Player for ScriptedPlayer {
    fn take_action(&self, _: PlayerState, _: ActionType) -> ActionData {
        let mut inner = self.0.lock().unwrap();
        inner.pop_front().unwrap()
    }
}

impl ScriptedPlayer {
    pub fn new<I: IntoIterator<Item = ActionData>>(actions: I) -> Self {
        Self(Mutex::new(actions.into_iter().collect()))
    }

    pub fn into_player(self) -> Arc<dyn Player> {
        Arc::new(self)
    }
}
