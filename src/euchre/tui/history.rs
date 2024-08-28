//! Widget for the history prompt

use std::collections::HashMap;
use std::iter::FromIterator;

use itertools::Itertools;
use ratatui::layout::Offset;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, ListState, Padding, Widget};
use tree::PreorderNode;

use crate::euchre::{Action, ActionData, ActionType, Log, LogId, Seat};

mod tree;
use self::tree::{Id as TreeId, Tree};

const VERT: char = '│';
const VERT_RIGHT: char = '├';
const ARC_UP_RIGHT: char = '╰';

pub type HistoryState = ListState;

/// A widget for displaying a tree-structured history of actions.
#[derive(Debug, Clone)]
pub struct History {
    items: Vec<Prefixed<HistoryItem>>,
}

/// A history item.
#[derive(Debug, Clone)]
enum HistoryItem {
    /// The deal. Always first.
    Deal { dealer: Seat },
    /// Actions that have been recorded in the log.
    Action {
        id: LogId,
        parent: Option<LogId>,
        action: Action,
    },
    /// The cursor position at the time the history widget was opened.
    Cursor { parent: Option<LogId> },
}

impl HistoryItem {
    /// The parent log ID for this node, if applicable
    fn parent(&self) -> Option<LogId> {
        match self {
            Self::Deal { .. } => None,
            Self::Action { parent, .. } | Self::Cursor { parent, .. } => *parent,
        }
    }
}

/// Helper function to build a tree out of a log.
fn build_tree(cursor: Option<LogId>, log: &Log) -> Tree<HistoryItem> {
    let mut builder = Tree::builder();
    let mut id_map: HashMap<Option<LogId>, TreeId> = HashMap::new();
    let mut parents: Vec<(TreeId, Option<LogId>)> = vec![];

    // Always insert the `Deal` node.
    let dealer = log.config().dealer();
    let root = builder.insert(HistoryItem::Deal { dealer });
    id_map.insert(None, root);

    // Insert all actions, sorted by sequence number.
    for node in log.action_nodes().sorted_unstable_by_key(|n| n.id) {
        let id = builder.insert(HistoryItem::Action {
            id: node.id,
            parent: node.parent,
            action: node.action,
        });
        parents.push((id, node.parent));
        id_map.insert(Some(node.id), id);
    }

    // Always insert the `Cursor` node.
    let id = builder.insert(HistoryItem::Cursor { parent: cursor });
    parents.push((id, cursor));

    // Now that all nodes have been assigned IDs, we can set the parents.
    for (id, parent) in parents {
        let parent = *id_map.get(&parent).expect("consistency");
        builder.set_parent(id, parent);
    }

    builder.build()
}

/// Helper structure for assigning prefixes to nodes in the tree.
#[derive(Default)]
struct PrefixHelper {
    /// The base prefix for a node, indexed by parent node.
    base: HashMap<LogId, String>,
}
impl PrefixHelper {
    /// Look up the base prefix for a parent node.
    fn base(&self, id: Option<LogId>) -> &str {
        id.and_then(|id| self.base.get(&id))
            .map_or("", |s| s.as_str())
    }

    /// Calculate the prefix for a node, and update the base prefix for its children.
    fn prefix(&mut self, node: &PreorderNode<'_, HistoryItem>) -> String {
        let (id, parent) = match node.data {
            HistoryItem::Deal { .. } => return String::new(),
            HistoryItem::Action { id, parent, .. } => (Some(*id), *parent),
            HistoryItem::Cursor { parent } => (None, *parent),
        };
        let base = self.base(parent);
        let (prefix, next_base) = if node.last_sibling {
            (format!("{base}{ARC_UP_RIGHT} "), Some(format!("{base}  ")))
        } else if node.sibling {
            (
                format!("{base}{VERT_RIGHT} "),
                Some(format!("{base}{VERT} ")),
            )
        } else if node.leaf {
            (format!("{base}{ARC_UP_RIGHT} "), None)
        } else {
            (format!("{base}{VERT} "), Some(base.to_string()))
        };
        if let Some((id, base)) = id.zip(next_base) {
            self.base.insert(id, base);
        }
        prefix
    }
}

impl History {
    /// Creates a new history widget.
    pub fn new(cursor: Option<LogId>, log: &Log) -> Self {
        let tree = build_tree(cursor, log);
        let mut items = vec![];
        let mut helper = PrefixHelper::default();
        for node in tree.preorder() {
            let prefix = helper.prefix(&node);
            items.push(Prefixed::new(prefix, node.data.clone()));
        }
        Self { items }
    }

    /// Returns the index of the cursor item.
    pub fn cursor_position(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|item| matches!(item.inner(), HistoryItem::Cursor { .. }))
    }

    /// Returns the selected log entry in the history. Note that the log entry pertaining to the
    /// initial deal is `None`, which will be returned as `Some(None)` when selected.
    #[allow(clippy::option_option)]
    pub fn selected(&self, state: &HistoryState) -> Option<Option<LogId>> {
        state
            .selected()
            .and_then(|idx| self.items.get(idx))
            .map(|item| item.inner().parent())
    }

    /// Determines the indexes of the first and last item to be displayed, given the height
    /// of the rendering area.
    fn get_item_bounds(&self, state: &HistoryState, height: usize) -> (usize, usize) {
        const PADDING: usize = 1;
        let max = self.items.len().saturating_sub(1);
        let delta = height.saturating_sub(1);
        let offset = state.offset().min(max);
        let first = offset;
        let last = (offset + delta).min(max);
        match state.selected() {
            Some(index) if index < first + PADDING => {
                let first = index.saturating_sub(PADDING);
                (first, (first + delta).min(max))
            }
            Some(index) if index + PADDING > last => {
                let last = (index + PADDING).min(max);
                (last.saturating_sub(delta), last)
            }
            _ => (first, last),
        }
    }
}

/// Helper function for translating an [`Action`] into a collection of [`Span`]s.
fn action_spans(action: Action) -> Vec<Span<'static>> {
    let mut spans = vec![Span::from(action.seat.to_string())];
    match (action.action, action.data) {
        (_, ActionData::Pass) => spans.push(" passed".into()),
        (_, ActionData::Call { suit, alone }) => {
            spans.extend([" called ".into(), suit.to_span()]);
            if alone {
                spans.push(" alone".into());
            }
        }
        (ActionType::DealerDiscard, ActionData::Card { card }) => {
            spans.extend([" discarded ".into(), card.to_span()]);
        }
        (ActionType::Lead, ActionData::Card { card }) => {
            spans.extend([" led ".into(), card.to_span()]);
        }
        (ActionType::Follow, ActionData::Card { card }) => {
            spans.extend([" followed ".into(), card.to_span()]);
        }
        _ => unreachable!(),
    }
    spans
}

trait IntoSpans {
    /// Converts the item into a list of spans.
    fn into_spans(self) -> Vec<Span<'static>>;
}

impl IntoSpans for HistoryItem {
    fn into_spans(self) -> Vec<Span<'static>> {
        match self {
            Self::Deal { dealer } => vec![format!("{dealer} dealt").into()],
            Self::Action { action, .. } => action_spans(action),
            Self::Cursor { .. } => vec!["(you are here)".into()],
        }
    }
}

/// A wrapper for attaching a prefix to a list item.
#[derive(Debug, Clone)]
struct Prefixed<T> {
    prefix: String,
    inner: T,
}

impl<T> Prefixed<T> {
    fn new(prefix: String, inner: T) -> Self {
        Self { prefix, inner }
    }

    fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T: IntoSpans> Prefixed<T> {
    fn into_line(self, selected: bool) -> Line<'static> {
        let mut spans = self.inner.into_spans();
        if selected {
            spans = spans.into_iter().map(Span::reversed).collect();
        }
        spans.insert(0, Span::raw(self.prefix));
        Line::from_iter(spans)
    }
}

impl Widget for History {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let mut state = HistoryState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

impl StatefulWidget for History {
    type State = HistoryState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::new().padding(Padding::left(1));
        let list_area = block.inner(area);
        block.render(area, buf);
        if list_area.is_empty() {
            return;
        }
        if let Some(index) = state.selected() {
            state.select(Some(index.max(1).min(self.items.len() - 1)));
        }

        let list_height = list_area.height as usize;
        let (first_index, last_index) = self.get_item_bounds(state, list_height);
        *(state.offset_mut()) = first_index;

        let mut item_area = Rect::new(list_area.x, list_area.y, list_area.width, 1);
        for (i, item) in self
            .items
            .into_iter()
            .enumerate()
            .skip(first_index)
            .take(last_index - first_index + 1)
        {
            let selected = state.selected().is_some_and(|s| s == i);
            let line = item.into_line(selected);
            line.render(item_area, buf);
            item_area = item_area.offset(Offset { x: 0, y: 1 });
        }
    }
}
