//! Widget for the history prompt

use std::collections::HashMap;
use std::iter::FromIterator;

use ratatui::layout::Offset;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, ListState, Padding, Widget};

use crate::euchre::{Action, ActionData, ActionType, Log, LogId, Seat};

pub type HistoryState = ListState;

#[derive(Debug, Clone)]
pub struct History {
    items: Vec<HistoryItem>,
}

const VERT: char = '│';
const VERT_RIGHT: char = '├';
const ARC_UP_RIGHT: char = '╰';

impl History {
    pub fn new(cursor: Option<LogId>, log: &Log) -> Self {
        let dealer = log.config().dealer();
        let mut items = vec![HistoryItem::Deal { dealer }];

        if cursor.is_none() {
            // Should be VERT_RIGHT if there are siblings.
            items.push(HistoryItem::cursor(cursor, format!("{ARC_UP_RIGHT} ")));
        }

        let mut child_prefixes: HashMap<LogId, String> = HashMap::new();
        for node in log.traverse() {
            let mut prefix = node
                .parent
                .and_then(|id| child_prefixes.get(&id))
                .cloned()
                .unwrap_or_default();
            let (char, child_prefix) = if node.last_sibling {
                (ARC_UP_RIGHT, format!("{prefix}  "))
            } else if node.sibling {
                (VERT_RIGHT, format!("{prefix}{VERT} "))
            } else if node.leaf {
                (ARC_UP_RIGHT, format!("{prefix}  "))
            } else {
                (VERT, prefix.clone())
            };
            prefix.push(char);
            prefix.push(' ');
            child_prefixes.insert(node.id, child_prefix);
            items.push(HistoryItem::action(node.parent, node.action, &prefix));

            if cursor.is_some_and(|id| id == node.id) {
                let child_prefix = child_prefixes.get(&node.id).expect("just inserted");
                // Should be VERT_RIGHT if there are siblings.
                let prefix = format!("{child_prefix}{ARC_UP_RIGHT} ");
                items.push(HistoryItem::cursor(cursor, prefix));
            }
        }

        Self { items }
    }

    pub fn position(&self, id: Option<LogId>) -> Option<usize> {
        self.items.iter().position(|item| item.parent() == id)
    }

    /// Returns the selected log entry in the history. Note that the log entry pertaining to the
    /// initial deal is `None`, which will be returned as `Some(None)` when selected.
    #[allow(clippy::option_option)]
    pub fn selected(&self, state: &HistoryState) -> Option<Option<LogId>> {
        state
            .selected()
            .and_then(|idx| self.items.get(idx))
            .map(HistoryItem::parent)
    }

    fn get_item_bounds(&self, state: &HistoryState, height: usize) -> (usize, usize) {
        const PADDING: usize = 1;
        let max = self.items.len().saturating_sub(1);
        let delta = height.saturating_sub(1);
        let offset = state.offset().min(max);
        let first = offset;
        let last = (offset + delta).min(max);
        match state.selected() {
            Some(index) if index <= first => {
                let first = index.saturating_sub(PADDING);
                (first, (first + delta).min(max))
            }
            Some(index) if index >= last => {
                let last = (index + PADDING).min(max);
                (last.saturating_sub(delta), last)
            }
            _ => (first, last),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HistoryItem {
    Deal {
        dealer: Seat,
    },
    Action {
        parent: Option<LogId>,
        action: Action,
        prefix: String,
    },
    Cursor {
        parent: Option<LogId>,
        prefix: String,
    },
}

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

impl HistoryItem {
    pub fn action<S: Into<String>>(parent: Option<LogId>, action: Action, prefix: S) -> Self {
        Self::Action {
            parent,
            action,
            prefix: prefix.into(),
        }
    }

    pub fn cursor<S: Into<String>>(parent: Option<LogId>, prefix: S) -> Self {
        Self::Cursor {
            parent,
            prefix: prefix.into(),
        }
    }

    pub fn parent(&self) -> Option<LogId> {
        match self {
            Self::Deal { .. } => None,
            Self::Action { parent, .. } | Self::Cursor { parent, .. } => *parent,
        }
    }

    pub fn line(self, selected: bool) -> Line<'static> {
        match self {
            Self::Deal { dealer } => {
                let line = Line::from(format!("{dealer} dealt"));
                if selected {
                    line.reversed()
                } else {
                    line
                }
            }
            Self::Action { action, prefix, .. } => {
                let mut spans = action_spans(action);
                if selected {
                    spans = spans.into_iter().map(Span::reversed).collect();
                }
                spans.insert(0, Span::raw(prefix));
                Line::from(spans)
            }
            Self::Cursor { prefix, .. } => {
                let mut span = Span::raw("(you are here)");
                if selected {
                    span = span.reversed();
                }
                Line::from_iter([prefix.into(), span])
            }
        }
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
            let line = item.line(selected);
            line.render(item_area, buf);
            item_area = item_area.offset(Offset { x: 0, y: 1 });
        }
    }
}
