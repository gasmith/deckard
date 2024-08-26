//! Tree-structured log of actions for a round.

use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::RoundConfig;
use crate::euchre::{Action, RoundError};

#[cfg(test)]
mod test;

pub type Id = u32;

/// A node in the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ActionNode {
    /// The ID for this node.
    id: Id,
    /// The parent of this node.
    parent: Option<Id>,
    /// The action that this node represents.
    action: Action,
}
impl ActionNode {
    /// Creates a new [`ActionNode`].
    fn new(id: Id, parent: Option<Id>, action: Action) -> Self {
        Self { id, parent, action }
    }
}

/// A serializable version of the log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawLog {
    /// The initial configuration.
    config: RoundConfig,
    /// An unordered list of nodes in the action tree.
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

/// A tree-structured log of actions taken in a round.
#[derive(Debug, Clone)]
pub struct Log {
    /// The initial configuration for the round.
    config: RoundConfig,
    /// A map of actions, indexed by node ID.
    actions: HashMap<Id, ActionNode>,
    /// A map of children, indexed by an `Option<Id>`. The initial state, immediately after the deal, is
    /// represented by `None`. All other actions are represented by `Some(id)`.
    children: HashMap<Option<Id>, Vec<Id>>,
    /// The next ID to use when adding a new action to the log.
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
    /// Creates a new [`Log`] with the specified initial configuration.
    pub fn new(config: RoundConfig) -> Self {
        Self {
            config,
            actions: HashMap::default(),
            children: HashMap::default(),
            next_id: 0,
        }
    }

    /// Returns an immutable reference to the initial configuration.
    pub fn config(&self) -> &RoundConfig {
        &self.config
    }

    /// Finds a child of the specified node with a matching action.
    fn find_child(&self, parent: Option<Id>, action: Action) -> Option<Id> {
        self.children
            .get(&parent)
            .and_then(|ids| ids.iter().find(|id| self.actions[id].action == action))
            .copied()
    }

    /// Inserts an action into the log. If the same action is present under the same parent, this
    /// function is a no-op.
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

    /// Returns a backtrace of actions from the specified ID, back to the very first action.
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

    /// Returns an iterator to traverse the tree in preorder.
    pub fn traverse(&self) -> Traverse<'_> {
        Traverse::new(self)
    }
}

/// A node in the pre-order traversal of the tree.
pub struct TraverseNode<'a> {
    /// The node ID.
    pub id: Id,
    /// The action this node represents.
    pub action: Action,
    /// The parent of this node.
    pub parent: Option<Id>,
    /// Set to `true` if the node has other siblings.
    pub sibling: bool,
    /// Set to `true` if the node has other siblings, and this is the last one in the traversal.
    pub last_sibling: bool,
    /// Set to `true` if this node has no children.
    pub leaf: bool,
    /// Binds a lifetime to the `&`[`Log`] over which we're iterating.
    phantom: PhantomData<&'a ()>,
}

/// Iterator state for pre-order traversal of the tree.
pub struct Traverse<'a> {
    /// The log we're traversing.
    log: &'a Log,
    /// A queue of nodes to traverse.
    queue: VecDeque<Id>,
}

impl<'a> Iterator for Traverse<'a> {
    type Item = TraverseNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.queue.pop_front()?;
        let action = self.log.actions.get(&id).expect("consistency");
        let siblings = self.log.children.get(&action.parent).expect("consistency");
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
    /// Creates a new [`Traverse`] iterator.
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
