//! Widget for the play arena

use ratatui::widgets::{Block, Widget};
use ratatui::{prelude::*, widgets::Paragraph};

use crate::game::euchre::{ActionType, Card, ExpectAction, Seat, Trick};

use super::{PlayerState, Wait};

pub struct Arena<'a> {
    wait: &'a Wait,
    state: &'a PlayerState<'a>,
}

impl<'a> Arena<'a> {
    pub fn new(wait: &'a Wait, state: &'a PlayerState<'a>) -> Self {
        Self { wait, state }
    }

    fn top_card_span(&self, top: Option<Card>) -> Span<'_> {
        top.map(|c| c.to_span()).unwrap_or(Span::raw("  "))
    }

    fn trick_card_span(&self, trick: Option<&Trick>, seat: Seat) -> Span<'_> {
        trick
            .and_then(|t| t.get_card(seat))
            .map(|c| c.to_span())
            .unwrap_or(Span::raw("  "))
    }

    fn to_lines(&self) -> Vec<Line<'_>> {
        let top = match self.wait {
            Wait::Deal
            | Wait::Action(ExpectAction {
                action: ActionType::BidTop | ActionType::DealerDiscard,
                ..
            }) => Some(self.state.top),
            _ => None,
        };
        let trick = match &self.wait {
            Wait::Action(ExpectAction {
                action: ActionType::Follow,
                ..
            }) => self.state.current_trick(),
            Wait::Trick(trick) => Some(trick),
            _ => None,
        };
        vec![
            Span::raw("N").into_centered_line(),
            Line::default(),
            self.trick_card_span(trick, Seat::North)
                .into_centered_line(),
            Line::from(vec![
                Span::raw("W  "),
                self.trick_card_span(trick, Seat::West),
                self.top_card_span(top),
                self.trick_card_span(trick, Seat::East),
                Span::raw("  E"),
            ])
            .centered(),
            self.trick_card_span(trick, Seat::South)
                .into_centered_line(),
            Line::default(),
            Span::raw("S").into_centered_line(),
        ]
    }
}

impl<'a> Widget for Arena<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let lines = self.to_lines();
        Paragraph::new(lines)
            .block(Block::bordered())
            .render(area, buf)
    }
}
