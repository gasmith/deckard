//! Informational widget

use std::iter::FromIterator;

use ratatui::{
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use crate::euchre::{Contract, Event, ExpectAction, Game, GameOutcome, Round, RoundOutcome, Seat};

use super::Mode;

enum First {
    Dealer(Seat),
    Contract(Contract),
    Empty,
}
impl First {
    fn into_line(self) -> Line<'static> {
        match self {
            Self::Dealer(dealer) => format!("{dealer}.").into(),
            Self::Contract(contract) => Line::from_iter([
                format!("{} called ", contract.maker).into(),
                contract.suit.to_span(),
                if contract.alone { " alone." } else { "." }.into(),
            ]),
            Self::Empty => Line::default(),
        }
    }
}

enum Second {
    Event(Event),
    Expect(ExpectAction),
    Empty,
}
impl Second {
    fn into_line(self) -> Line<'static> {
        match self {
            Self::Event(Event::Trick(trick)) => {
                format!("{} takes the trick.", trick.best().0).into()
            }
            Self::Event(Event::Round(RoundOutcome { team, points })) => {
                format!("{} win {points} points.", team.to_abbr()).into()
            }
            Self::Event(Event::Game(GameOutcome { team, .. })) => {
                format!("{} wins the game.", team.to_abbr()).into()
            }
            Self::Expect(ExpectAction { seat, action }) => format!("{seat} to {action}.").into(),
            _ => Line::default(),
        }
    }
}

pub struct Info(First, Second);

impl Info {
    pub fn new<R: Round>(mode: &Mode, game: &Game<R>) -> Self {
        let round = game.round();

        let first = match (mode, round.contract()) {
            (Mode::Event(Event::Game(_)), _) => First::Empty,
            (_, Some(contract)) => First::Contract(contract),
            (_, None) => First::Dealer(round.dealer()),
        };

        let second = match (mode, round.next_action()) {
            (Mode::Event(event), _) => Second::Event(event.clone()),
            (_, Some(expect)) => Second::Expect(expect),
            _ => Second::Empty,
        };

        Self(first, second)
    }
}

impl Widget for Info {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Paragraph::new(Text::from_iter([self.0.into_line(), self.1.into_line()]))
            .block(Block::bordered())
            .render(area, buf);
    }
}
