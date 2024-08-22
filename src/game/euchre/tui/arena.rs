//! Widget for the play arena

use ratatui::widgets::{Block, Widget};
use ratatui::{prelude::*, widgets::Paragraph};

use crate::game::euchre::{ActionType, Card, Event, Round, Seat, Trick};

use super::Mode;

pub struct Arena {
    top: Option<Card>,
    trick: Option<Trick>,
}

impl Arena {
    pub fn new(mode: &Mode, round: &impl Round) -> Self {
        let action = round.next_action().map(|expect| expect.action);
        let top = match (mode, action) {
            (Mode::Event(Event::Deal(_, _)), _) | (_, Some(ActionType::BidTop)) => {
                Some(round.top_card())
            }
            _ => None,
        };
        let trick = match (mode, action) {
            (Mode::Event(Event::Trick(trick)), _) => Some(trick.clone()),
            (_, Some(ActionType::Follow)) => round.tricks().last().cloned(),
            _ => None,
        };
        Self { top, trick }
    }

    fn top_card_span(&self) -> Span<'_> {
        self.top.map(|c| c.to_span()).unwrap_or(Span::raw("  "))
    }

    fn trick_card_span(&self, seat: Seat) -> Span<'_> {
        self.trick
            .as_ref()
            .and_then(|t| t.get_card(seat))
            .map(|c| c.to_span())
            .unwrap_or(Span::raw("  "))
    }

    fn to_lines(&self) -> Vec<Line<'_>> {
        vec![
            Span::raw("N").into_centered_line(),
            Line::default(),
            self.trick_card_span(Seat::North).into_centered_line(),
            Line::from(vec![
                Span::raw("W  "),
                self.trick_card_span(Seat::West),
                self.top_card_span(),
                self.trick_card_span(Seat::East),
                Span::raw("  E"),
            ])
            .centered(),
            self.trick_card_span(Seat::South).into_centered_line(),
            Line::default(),
            Span::raw("S").into_centered_line(),
        ]
    }
}

impl Widget for Arena {
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
