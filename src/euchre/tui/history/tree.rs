//! A helper module for a preorder traversal of the history tree.

use std::collections::{HashMap, VecDeque};

use itertools::Itertools;

pub type Id = u32;

/// A builder for constructing the tree.
pub struct Builder<T> {
    next_id: Id,
    nodes: HashMap<Id, Node<T>>,
}
impl<T> Default for Builder<T> {
    fn default() -> Self {
        Self {
            next_id: Default::default(),
            nodes: HashMap::default(),
        }
    }
}
impl<T> Builder<T> {
    /// Inserts a node into the tree.
    pub fn insert(&mut self, data: T) -> Id {
        let id = self.next_id;
        self.nodes.insert(id, Node::new(id, data));
        self.next_id += 1;
        id
    }

    /// Sets the node's parent.
    pub fn set_parent(&mut self, node: Id, parent: Id) {
        assert!(self.nodes.contains_key(&parent));
        self.nodes.get_mut(&node).unwrap().parent = Some(parent);
    }
}
impl<T> From<Builder<T>> for Tree<T> {
    fn from(builder: Builder<T>) -> Self {
        let count = builder.nodes.len();
        let mut roots = vec![];
        let mut nodes = HashMap::with_capacity(count);
        let mut children: HashMap<_, Vec<_>> = HashMap::with_capacity(count);
        for node in builder.nodes.into_values() {
            if let Some(parent) = node.parent {
                children.entry(parent).or_default().push(node.id);
            } else {
                roots.push(node.id);
            }
            let prev = nodes.insert(node.id, node);
            assert!(prev.is_none());
        }
        assert!(children.keys().all(|id| nodes.contains_key(id)));
        Tree {
            roots,
            nodes,
            children,
        }
    }
}

/// A simple tree structure.
pub struct Tree<T> {
    /// Root node.
    roots: Vec<Id>,

    /// Map of nodes.
    nodes: HashMap<Id, Node<T>>,

    /// Children for each node.
    children: HashMap<Id, Vec<Id>>,
}

/// A node in the tree.
#[derive(Debug, Clone)]
struct Node<T> {
    /// The node ID.
    id: Id,
    /// The parent node ID, if this is not a root.
    parent: Option<Id>,
    /// The inner data for this node.
    data: T,
}

impl<T> Node<T> {
    /// Creates a new node.
    fn new(id: Id, data: T) -> Self {
        Self {
            id,
            parent: None,
            data,
        }
    }
}

/// Iterator state for pre-order traversal of the tree.
pub struct Preorder<'a, T> {
    /// The log we're traversing.
    tree: &'a Tree<T>,
    /// A queue of nodes to traverse.
    queue: VecDeque<Id>,
}

/// A node in the pre-order traversal of the tree.
pub struct PreorderNode<'a, T> {
    /// The node.
    pub data: &'a T,
    /// Set to `true` if the node has other siblings.
    pub sibling: bool,
    /// Set to `true` if the node has other siblings, and this is the last one in the traversal.
    pub last_sibling: bool,
    /// Set to `true` if this node has no children.
    pub leaf: bool,
}

impl<T> Tree<T> {
    /// Creates a new builder.
    pub fn builder() -> Builder<T> {
        Builder::default()
    }

    /// Returns the specified node.
    fn get(&self, id: Id) -> Option<&Node<T>> {
        self.nodes.get(&id)
    }

    /// Returns a list of child IDs for the specified node.
    fn get_children(&self, id: Id) -> Option<&Vec<u32>> {
        self.children.get(&id)
    }

    /// Creates a new iterator for a preorder traversal of the tree.
    pub fn preorder(&self) -> Preorder<'_, T> {
        let queue = self.roots.iter().copied().sorted_unstable().collect();
        Preorder { tree: self, queue }
    }
}

impl<'a, T> Iterator for Preorder<'a, T> {
    type Item = PreorderNode<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.queue.pop_front()?;
        let node = self.tree.get(id).expect("consistency");
        let siblings = node
            .parent
            .map(|id| self.tree.get_children(id).expect("consistency"))
            .cloned()
            .unwrap_or_default();
        let (sibling, last_sibling) = if siblings.len() > 1 {
            (true, siblings.iter().max().is_some_and(|m| *m == id))
        } else {
            (false, false)
        };
        let children = self.tree.get_children(id);
        if let Some(children) = children {
            // Visit children in ascending order.
            for id in children.iter().sorted_unstable().rev() {
                self.queue.push_front(*id);
            }
        }
        Some(PreorderNode {
            data: &node.data,
            sibling,
            last_sibling,
            leaf: children.is_none(),
        })
    }
}
