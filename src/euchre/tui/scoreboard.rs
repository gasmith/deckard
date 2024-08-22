use ratatui::widgets::{Block, Row, Table, Widget};

use crate::euchre::{Game, Round, Team};

pub struct Scoreboard {
    ns_score: u8,
    ew_score: u8,
    ns_tricks: u8,
    ew_tricks: u8,
}

impl Scoreboard {
    pub fn new<R: Round>(game: &Game<R>) -> Self {
        let ns_score = game.score(Team::NorthSouth);
        let ew_score = game.score(Team::EastWest);
        let tricks = game.round().tricks();
        let ns_tricks = tricks.win_count(Team::NorthSouth);
        let ew_tricks = tricks.win_count(Team::EastWest);
        Self {
            ns_score,
            ew_score,
            ns_tricks,
            ew_tricks,
        }
    }
}

impl Widget for Scoreboard {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Table::default()
            .header(Row::new(["", "N/S", "E/W"]))
            .rows([
                Row::new([
                    String::from("Score"),
                    self.ns_score.to_string(),
                    self.ew_score.to_string(),
                ]),
                Row::new([
                    String::from("Trick"),
                    self.ns_tricks.to_string(),
                    self.ew_tricks.to_string(),
                ]),
            ])
            .block(Block::bordered())
            .render(area, buf)
    }
}
