use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::euchre::{Action, RoundError};

use super::RoundConfig;

#[cfg(test)]
mod test;

pub type Id = u32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawLog {
    config: RoundConfig,
    actions: Vec<ActionNode>,
}
impl From<Log> for RawLog {
    fn from(log: Log) -> Self {
        RawLog {
            config: log.config,
            actions: log
                .actions
                .into_values()
                .sorted_unstable_by_key(|a| a.id)
                .collect(),
        }
    }
}
impl<'a> From<&'a Log> for RawLog {
    fn from(log: &'a Log) -> Self {
        RawLog {
            config: log.config.clone(),
            actions: log
                .actions
                .values()
                .sorted_unstable_by_key(|a| a.id)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Log {
    config: RoundConfig,
    actions: HashMap<Id, ActionNode>,
    children: HashMap<Option<Id>, Vec<Id>>,
    next_id: Id,
}
impl From<RawLog> for Log {
    fn from(raw: RawLog) -> Self {
        let mut max_id = 0;
        let mut children: HashMap<_, Vec<_>> = HashMap::new();
        let mut actions = HashMap::new();
        for action in raw.actions {
            if action.id > max_id {
                max_id = action.id;
            }
            children.entry(action.parent).or_default().push(action.id);
            actions.insert(action.id, action);
        }
        Log {
            config: raw.config,
            actions,
            children,
            next_id: max_id + 1,
        }
    }
}

impl Log {
    pub fn new(config: RoundConfig) -> Self {
        Self {
            config,
            actions: HashMap::default(),
            children: HashMap::default(),
            next_id: 0,
        }
    }

    pub fn config(&self) -> &RoundConfig {
        &self.config
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
            let prev = self.actions.insert(node.id, node);
            assert!(prev.is_none());
            self.children.entry(parent).or_default().push(id);
            id
        });
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

    pub fn traverse(&self) -> impl Iterator<Item = TraverseNode<'_>> {
        Traverse::new(self)
    }
}

pub struct TraverseNode<'a> {
    pub id: Id,
    pub action: Action,
    pub parent: Option<Id>,
    pub sibling: bool,
    pub last_sibling: bool,
    pub leaf: bool,
    phantom: PhantomData<&'a ()>,
}

pub struct Traverse<'a> {
    log: &'a Log,
    queue: VecDeque<Id>,
}

impl<'a> Iterator for Traverse<'a> {
    type Item = TraverseNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.queue.pop_front()?;
        let action = self.log.actions.get(&id).unwrap();
        let siblings = self.log.children.get(&action.parent).unwrap();
        let (sibling, last_sibling) = if siblings.len() > 1 {
            (true, siblings.iter().max().is_some_and(|m| *m == id))
        } else {
            (false, false)
        };
        let children = self.log.children.get(&Some(id));
        if let Some(children) = children {
            for id in children.iter().sorted_unstable().rev() {
                self.queue.push_front(*id);
            }
        }
        Some(TraverseNode {
            id,
            action: action.action,
            parent: action.parent,
            sibling,
            last_sibling,
            leaf: children.is_none(),
            phantom: PhantomData,
        })
    }
}

impl<'a> Traverse<'a> {
    fn new(log: &'a Log) -> Self {
        let queue = log
            .children
            .get(&None)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        Self { log, queue }
    }
}
