//! Widget for the history prompt
//!
//! TODO:
//!  - Add a "deal" pseudo-item
//!  - Tree structure!

use std::collections::HashMap;

use ratatui::layout::Offset;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, ListState, Padding, Widget};

use crate::euchre::{Action, ActionData, ActionType, Log, LogId};

pub type HistoryState = ListState;

#[derive(Debug, Clone)]
pub struct History {
    items: Vec<HistoryItem>,
}

const VERT: char = '│';
const VERT_RIGHT: char = '├';
const ARC_UP_RIGHT: char = '╰';

impl History {
    pub fn new(log: &Log) -> Self {
        let mut items = vec![HistoryItem::Deal];

        let mut child_prefixes: HashMap<LogId, String> = HashMap::new();
        for node in log.traverse() {
            let mut prefix = node
                .parent
                .and_then(|id| child_prefixes.get(&id))
                .cloned()
                .unwrap_or_default();
            if node.last_sibling {
                child_prefixes.insert(node.id, format!("{prefix}  "));
                prefix.push(ARC_UP_RIGHT);
            } else if node.sibling {
                child_prefixes.insert(node.id, format!("{prefix}{VERT} "));
                prefix.push(VERT_RIGHT);
            } else if node.leaf {
                prefix.push(ARC_UP_RIGHT);
            } else {
                child_prefixes.insert(node.id, prefix.clone());
                prefix.push(VERT);
            }
            prefix.push(' ');
            items.push(HistoryItem::action(node.id, node.action, prefix))
        }

        Self { items }
    }

    pub fn position(&self, id: Option<LogId>) -> Option<usize> {
        self.items.iter().position(|item| item.id() == id)
    }

    pub fn selected(&self, state: &HistoryState) -> Option<Option<LogId>> {
        state
            .selected()
            .and_then(|idx| self.items.get(idx))
            .map(|item| item.id())
    }

    fn get_item_bounds(&self, state: &HistoryState, height: usize) -> (usize, usize) {
        let offset = state.offset().min(self.items.len().saturating_sub(1));
        let first_index = offset;
        let last_index = (offset + height).min(self.items.len().saturating_sub(1));
        // TODO: selected must fall within the range (first, last).
        (first_index, last_index)
    }
}

#[derive(Debug, Clone)]
pub enum HistoryItem {
    Deal,
    Action {
        id: LogId,
        action: Action,
        prefix: String,
    },
}

fn action_spans(action: &Action) -> Vec<Span<'static>> {
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
            // TODO: Redact card in certain modes?
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
    pub fn action(id: LogId, action: Action, prefix: String) -> Self {
        Self::Action { id, action, prefix }
    }

    pub fn id(&self) -> Option<LogId> {
        match self {
            Self::Deal => None,
            Self::Action { id, .. } => Some(*id),
        }
    }

    pub fn line(self, selected: bool) -> Line<'static> {
        match self {
            Self::Deal => {
                let line = Line::raw("Deal");
                if selected {
                    line.reversed()
                } else {
                    line
                }
            }
            Self::Action { action, prefix, .. } => {
                let mut spans = action_spans(&action);
                if selected {
                    spans = spans.into_iter().map(Span::reversed).collect();
                }
                spans.insert(0, Span::raw(prefix));
                Line::from(spans)
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
        StatefulWidget::render(self, area, buf, &mut state)
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
        if state.selected().is_some_and(|s| s >= self.items.len()) {
            state.select(Some(self.items.len() - 1));
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
