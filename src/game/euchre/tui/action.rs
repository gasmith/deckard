//! Multiple choice list of actions.

use ratatui::widgets::{ListItem, ListState};
use ratatui::{prelude::*, widgets::List};

use crate::game::euchre::{ActionData, Suit};

pub type ActionChoiceState = ListState;

#[derive(Debug, Clone)]
pub struct ActionChoice {
    choices: Vec<ActionData>,
}

impl ActionChoice {
    fn new(choices: Vec<ActionData>) -> Self {
        Self { choices }
    }

    pub fn bid_top(suit: Suit) -> Self {
        Self::new(vec![
            ActionData::Pass,
            ActionData::Call { suit, alone: false },
            ActionData::Call { suit, alone: true },
        ])
    }

    pub fn bid_other(top_suit: Suit) -> Self {
        let mut choices = vec![ActionData::Pass];
        for alone in [false, true] {
            for &suit in Suit::all_suits() {
                if suit != top_suit {
                    choices.push(ActionData::Call { suit, alone })
                }
            }
        }
        Self::new(choices)
    }

    pub fn len(&self) -> usize {
        self.choices.len()
    }

    pub fn selected(&self, state: &ActionChoiceState) -> Option<ActionData> {
        state
            .selected()
            .and_then(|idx| self.choices.get(idx))
            .copied()
    }

    fn list(self) -> List<'static> {
        List::new(self.choices)
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>")
    }
}

impl From<ActionData> for ListItem<'static> {
    fn from(action: ActionData) -> Self {
        let spans: Vec<Span> = match action {
            ActionData::Pass => vec!["Pass".into()],
            ActionData::Call { suit, alone } => vec![
                "Call ".into(),
                suit.to_span(),
                if alone { " alone" } else { "" }.into(),
            ],
            // Cards are selected with the [`Hand`] widget.
            _ => unreachable!(),
        };
        ListItem::new(Line::from(spans))
    }
}

impl Widget for ActionChoice {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Widget::render(self.list(), area, buf)
    }
}

impl StatefulWidget for ActionChoice {
    type State = ActionChoiceState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidget::render(self.list(), area, buf, state)
    }
}
