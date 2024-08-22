//! Widget for the history prompt

use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Padding, Widget};

use super::{Action, ActionData, ActionType, LogId};

pub type HistoryState = ListState;

#[derive(Debug, Clone)]
pub struct History {
    items: Vec<HistoryItem>,
}

impl History {
    pub fn new<I>(items: I) -> Self
    where
        I: IntoIterator<Item = (LogId, Action)>,
    {
        let items: Vec<_> = items
            .into_iter()
            .map(|(id, action)| HistoryItem::new(id, action))
            .collect();
        Self { items }
    }

    pub fn list(self) -> List<'static> {
        let block = Block::new().padding(Padding::left(1));
        List::new(self.items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>")
    }

    pub fn selected(&self, state: &HistoryState) -> Option<LogId> {
        state
            .selected()
            .and_then(|idx| self.items.get(idx))
            .map(|item| item.id)
    }
}

#[derive(Debug, Clone)]
pub struct HistoryItem {
    id: LogId,
    action: Action,
}

impl HistoryItem {
    pub fn new(id: LogId, action: Action) -> Self {
        Self { id, action }
    }
}

impl<'a> From<HistoryItem> for ListItem<'a> {
    fn from(HistoryItem { action, .. }: HistoryItem) -> Self {
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
        Line::from(spans).into()
    }
}

impl Widget for History {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Widget::render(self.list(), area, buf)
    }
}

impl StatefulWidget for History {
    type State = HistoryState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidget::render(self.list(), area, buf, state)
    }
}
