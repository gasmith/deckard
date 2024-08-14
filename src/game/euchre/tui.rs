use std::collections::HashMap;
use std::io::{self, stdout, Stdout};
use std::iter::FromIterator;

use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, ExecutableCommand};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph, Row, Table};

use crate::game::euchre::{PlayerState, Trick};

use super::{ActionType, Event, ExpectAction, LoggingRound, Outcome, Player, Robot, Seat, Team};

type Term = Terminal<CrosstermBackend<Stdout>>;

pub fn tui_init() -> io::Result<Term> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    Ok(terminal)
}

pub fn tui_restore() -> io::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

enum State {
    Wait(ActionType),
    Trick(Trick),
    Round(Outcome),
}

struct Game {
    round: LoggingRound,
    robot: Robot,
    score: HashMap<Team, u8>,
}
impl Game {
    fn new() -> Self {
        Self {
            round: LoggingRound::random(),
            robot: Robot::default(),
            score: [(Team::NorthSouth, 0), (Team::EastWest, 0)]
                .iter()
                .copied()
                .collect(),
        }
    }

    fn next(&mut self) -> Option<State> {
        loop {
            while let Some(e) = self.round.pop_event() {
                match e {
                    Event::Trick(t) => return Some(State::Trick(t)),
                    Event::Round(o) => return Some(State::Round(o)),
                    _ => (),
                }
            }
            if let Some(expect) = self.round.next() {
                if expect.seat == Seat::South {
                    return Some(State::Wait(expect.action));
                } else {
                    self.robot_play(expect);
                    continue;
                }
            }
            return None;
        }
    }

    fn robot_play(&mut self, expect: ExpectAction) {
        let state = self.round.player_state(expect.seat);
        let data = self.robot.take_action(state, expect.action);
        let action = expect.with_data(data);
        self.round
            .apply(action)
            .expect("robots don't make mistakes");
    }
}

/*
enum State {
    Init,
    Play(Game),
    GameOver(Game),
    ViewLog(Game),
}
*/

struct Areas {
    arena: Rect,
    score: Rect,
    contract: Rect,
    prompt: Rect,
}
impl Areas {
    fn new(frame: &Frame) -> Self {
        let [arena_info, prompt] = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(9), // arena & info
                Constraint::Min(4),    // hand, prompt, debug
            ],
        )
        .areas(frame.area());
        let [arena, info] = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(16), // arena
                Constraint::Length(24), // info
            ],
        )
        .areas(arena_info);
        let [score, contract] = Layout::new(
            Direction::Vertical,
            [Constraint::Length(5), Constraint::Length(3)],
        )
        .areas(info);
        Self {
            arena,
            score,
            contract,
            prompt,
        }
    }
}

pub struct Tui {
    game: Game,
    exit: bool,
}

impl Default for Tui {
    fn default() -> Self {
        Self {
            game: Game::new(),
            exit: false,
        }
    }
}

impl Tui {
    pub fn run(mut self, mut terminal: Term) -> anyhow::Result<()> {
        while !self.exit {
            if let Some(state) = self.game.next() {
                terminal.draw(|frame| self.render_frame(frame, &state))?;
                self.handle_events(&state)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame, g_state: &State) {
        let areas = Areas::new(frame);
        let p_state = self.game.round.player_state(Seat::South);
        frame.render_widget(self.arena_widget(g_state, &p_state), areas.arena);
        frame.render_widget(self.score_widget(&p_state), areas.score);
        frame.render_widget(self.contract_widget(&p_state), areas.contract);
        frame.render_widget(self.hand_widget(&p_state), areas.prompt);
    }

    fn handle_events(&mut self, state: &State) -> io::Result<()> {
        match event::read()? {
            event::Event::Key(k) if k.code == KeyCode::Char('q') => self.exit = true,
            event::Event::Key(_) => {
                if matches!(state, State::Wait(_)) {
                    if let Some(expect) = self.game.round.next() {
                        self.game.robot_play(expect);
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn arena_widget(&self, g_state: &State, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let lines = match g_state {
            State::Wait(ActionType::BidTop | ActionType::DealerDiscard) => [
                Span::raw("N").into_centered_line(),
                Line::default(),
                Line::default(),
                Line::from(vec![
                    Span::raw("W    "),
                    p_state.top.to_span(),
                    Span::raw("    E"),
                ])
                .centered(),
                Line::default(),
                Line::default(),
                Span::raw("S").into_centered_line(),
            ],
            State::Wait(ActionType::BidOther) | State::Wait(ActionType::Lead) | State::Round(_) => {
                [
                    Span::raw("N").into_centered_line(),
                    Line::default(),
                    Line::default(),
                    Span::raw("W          E").into_centered_line(),
                    Line::default(),
                    Line::default(),
                    Span::raw("S").into_centered_line(),
                ]
            }
            State::Wait(ActionType::Follow) => [
                Span::raw("N").into_centered_line(),
                Line::default(),
                p_trick_span(p_state, Seat::North).into_centered_line(),
                Line::from(vec![
                    Span::raw("W  "),
                    p_trick_span(p_state, Seat::West),
                    Span::raw("  "),
                    p_trick_span(p_state, Seat::East),
                    Span::raw("  E"),
                ])
                .centered(),
                p_trick_span(p_state, Seat::South).into_centered_line(),
                Line::default(),
                Span::raw("S").into_centered_line(),
            ],
            State::Trick(trick) => [
                Span::raw("N").into_centered_line(),
                Line::default(),
                trick_span(trick, Seat::North).into_centered_line(),
                Line::from(vec![
                    Span::raw("W  "),
                    trick_span(trick, Seat::West),
                    Span::raw("  "),
                    trick_span(trick, Seat::East),
                    Span::raw("  E"),
                ])
                .centered(),
                trick_span(trick, Seat::South).into_centered_line(),
                Line::default(),
                Span::raw("S").into_centered_line(),
            ],
        };
        Paragraph::new(Text::from_iter(lines)).block(Block::bordered())
    }

    fn score_widget(&self, p_state: &PlayerState<'_>) -> Table<'static> {
        fn get(map: &HashMap<Team, u8>, team: Team) -> String {
            map.get(&team).copied().unwrap_or_default().to_string()
        }
        let ns_score = get(&self.game.score, Team::NorthSouth);
        let ew_score = get(&self.game.score, Team::EastWest);
        let tricks = p_state.trick_counts();
        let ns_tricks = get(&tricks, Team::NorthSouth);
        let ew_tricks = get(&tricks, Team::EastWest);
        Table::default()
            .header(Row::new(["", "N/S", "E/W"]))
            .rows([
                Row::new([String::from("Score"), ns_score, ew_score]),
                Row::new([String::from("Trick"), ns_tricks, ew_tricks]),
            ])
            .block(Block::bordered())
    }

    fn contract_widget(&self, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let s = if let Some(c) = p_state.contract {
            format!(
                "{} called {}{}",
                c.maker,
                c.suit,
                if c.alone { " alone" } else { "" }
            )
        } else {
            format!("{} dealt", p_state.dealer)
        };
        Paragraph::new(s).block(Block::bordered())
    }

    fn hand_widget(&self, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let mut spans = vec![Span::raw("South's hand: ")];
        let mut cards = p_state.hand.clone();
        if let Some(contract) = p_state.contract {
            cards.sort_unstable_by_key(|c| {
                (c.effective_suit(contract.suit), c.value(contract.suit, *c))
            });
        } else {
            cards.sort_unstable_by_key(|c| (c.suit, c.rank));
        }
        for c in cards {
            spans.push(c.to_span());
            spans.push(" ".into());
        }
        Paragraph::new(Line::from(spans)).block(Block::bordered())
    }
}

fn trick_span(trick: &Trick, seat: Seat) -> Span<'static> {
    trick
        .get_card(seat)
        .map(|c| c.to_span())
        .unwrap_or(Span::raw("  "))
}

fn p_trick_span(p_state: &PlayerState<'_>, seat: Seat) -> Span<'static> {
    p_state
        .current_trick()
        .map(|t| trick_span(t, seat))
        .unwrap_or(Span::raw("  "))
}
