//! Hand widget

use ratatui::prelude::*;
use ratatui::widgets::ListState;

use crate::game::euchre::{Card, Seat};

pub type HandState = ListState;

#[derive(Debug, Clone)]
pub struct Hand {
    seat: Seat,
    cards: Vec<Card>,
}

impl Hand {
    pub fn new<I>(seat: Seat, cards: I) -> Self
    where
        I: IntoIterator<Item = Card>,
    {
        let cards: Vec<_> = cards.into_iter().collect();
        Self { seat, cards }
    }

    pub fn selected(&self, state: &HandState) -> Option<Card> {
        state
            .selected()
            .and_then(|idx| self.cards.get(idx).copied())
    }

    fn line(self, selected: Option<Card>) -> Line<'static> {
        let mut spans = vec![format!("{}'s hand: ", self.seat).into()];
        for card in self.cards {
            let mut card_span = card.to_span();
            if selected.is_some_and(|c| c == card) {
                card_span = card_span.reversed();
            }
            spans.push(card_span);
            spans.push(" ".into());
        }
        Line::from(spans)
    }
}

impl Widget for Hand {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        self.line(None).render(area, buf)
    }
}

impl StatefulWidget for Hand {
    type State = HandState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(index) = state.selected_mut() {
            if *index >= self.cards.len() {
                *index = self.cards.len() - 1;
            }
        } else {
            state.select(Some(0));
        }
        let selected = self.selected(state);
        self.line(selected).render(area, buf)
    }
}
