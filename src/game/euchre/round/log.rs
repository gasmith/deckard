use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::game::euchre::RoundError;

use super::{Action, InitialState};

pub type Id = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionNode {
    id: Id,
    parent: Option<Id>,
    action: Action,
}

impl ActionNode {
    fn new(id: Id, parent: Option<Id>, action: Action) -> Self {
        Self { id, parent, action }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLog {
    initial: InitialState,
    actions: Vec<ActionNode>,
}
impl From<Log> for RawLog {
    fn from(log: Log) -> Self {
        RawLog {
            initial: log.initial,
            actions: log
                .actions
                .into_values()
                .sorted_unstable_by_key(|a| a.id)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Log {
    initial: InitialState,
    actions: HashMap<Id, ActionNode>,
    children: HashMap<Option<Id>, Vec<Id>>,
    next_id: Id,
}
impl From<RawLog> for Log {
    fn from(raw: RawLog) -> Self {
        let mut children: HashMap<_, Vec<_>> = HashMap::new();
        let mut actions = HashMap::new();
        for action in raw.actions {
            children.entry(action.parent).or_default().push(action.id);
            actions.insert(action.id, action);
        }
        Log {
            initial: raw.initial,
            actions,
            children,
            next_id: 0,
        }
    }
}

impl Log {
    pub fn new(initial: InitialState) -> Self {
        Self {
            initial,
            actions: HashMap::default(),
            children: HashMap::default(),
            next_id: 0,
        }
    }

    pub fn into_raw(self) -> RawLog {
        self.into()
    }

    pub fn initial(&self) -> &InitialState {
        &self.initial
    }

    pub fn find_child(&self, parent: Option<Id>, action: Action) -> Option<Id> {
        self.children
            .get(&parent)
            .and_then(|ids| ids.iter().find(|id| self.actions[id].action == action))
            .copied()
    }

    pub fn insert(&mut self, parent: Option<Id>, action: Action) -> Id {
        let id = self.find_child(parent, action).unwrap_or_else(|| {
            let id = self.next_id;
            self.next_id += 1;
            let node = ActionNode::new(id, parent, action);
            self.actions.insert(node.id, node);
            id
        });
        self.children.entry(parent).or_default().push(id);
        id
    }

    pub fn backtrace(&self, id: Id) -> Result<Vec<(Id, Action)>, RoundError> {
        let mut parent = Some(id);
        let mut trace = vec![];
        while let Some(id) = parent {
            let action = self.actions.get(&id).ok_or(RoundError::InvalidLogId(id))?;
            trace.insert(0, (action.id, action.action));
            parent = action.parent;
        }
        Ok(trace)
    }
}
