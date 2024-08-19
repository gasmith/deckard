use std::collections::HashMap;
use std::io::{self, stdout, Stdout};
use std::iter::FromIterator;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, ExecutableCommand};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph, Row, Table};

use crate::game::euchre::{PlayerState, Trick};

use super::{
    Action, ActionData, ActionType, Event, ExpectAction, LoggingRound, Outcome, Player, Robot,
    Seat, Suit, Team,
};

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

enum Wait {
    Deal,
    Action(ExpectAction),
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

    fn robot_play(&mut self, expect: ExpectAction) {
        let state = self.round.player_state(expect.seat);
        let data = self.robot.take_action(state, expect.action);
        let action = expect.with_data(data);
        self.round
            .apply(action)
            .expect("robots don't make mistakes");
    }

    fn next_round(&mut self) {
        let outcome = self.round.outcome().expect("round must be over");
        let score = self.score.entry(outcome.team).or_default();
        *score += outcome.points;
        self.round = self.round.next_round();
    }
}

struct Areas {
    arena: Rect,
    score: Rect,
    info: Rect,
    prompt: Rect,
}
impl Areas {
    fn new(frame: &Frame) -> Self {
        let [arena_score_info, prompt] = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(9), // arena & score & info
                Constraint::Min(4),    // hand, prompt, debug
            ],
        )
        .areas(frame.area());
        let [arena, score_info] = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(16), // arena
                Constraint::Length(24), // score & info
            ],
        )
        .areas(arena_score_info);
        let [score, info] = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(5), // score
                Constraint::Length(4), // info
            ],
        )
        .areas(score_info);
        Self {
            arena,
            score,
            info,
            prompt,
        }
    }
}

#[derive(Debug, Clone)]
struct Prompt {
    choices: Vec<ActionData>,
    index: usize,
}
impl Prompt {
    fn new(choices: Vec<ActionData>) -> Self {
        assert!(!choices.is_empty());
        Self { choices, index: 0 }
    }

    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = ActionData>,
    {
        Self::new(iter.into_iter().collect())
    }

    fn next(&mut self) {
        if self.index < self.choices.len() - 1 {
            self.index += 1;
        }
    }

    fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    fn action_data(&self) -> ActionData {
        self.choices[self.index]
    }

    fn action(&self, expect: &ExpectAction) -> Action {
        expect.with_data(self.action_data())
    }
}

pub struct Tui {
    game: Game,
    prompt: Option<Prompt>,
    error: Option<String>,
    debug: Option<String>,
    exit: bool,
}

impl Default for Tui {
    fn default() -> Self {
        Self {
            game: Game::new(),
            prompt: None,
            error: None,
            debug: None,
            exit: false,
        }
    }
}

impl Tui {
    pub fn run(mut self, mut terminal: Term) -> anyhow::Result<()> {
        while !self.exit {
            if let Some(wait) = self.advance() {
                terminal.draw(|frame| self.render_frame(frame, &wait))?;
                self.handle_events(&wait)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn advance(&mut self) -> Option<Wait> {
        loop {
            while let Some(e) = self.game.round.pop_event() {
                match e {
                    Event::Deal(_, _) => return Some(Wait::Deal),
                    Event::Trick(t) => return Some(Wait::Trick(t)),
                    Event::Round(o) => return Some(Wait::Round(o)),
                    _ => (),
                }
            }
            if let Some(expect) = self.game.round.next() {
                if expect.seat == Seat::South {
                    if self.prompt.is_none() {
                        self.prompt = Some(match expect.action {
                            ActionType::BidTop => Prompt::new(vec![
                                ActionData::Pass,
                                ActionData::BidTop { alone: false },
                                ActionData::BidTop { alone: true },
                            ]),
                            ActionType::BidOther => {
                                let top_suit = self.game.round.player_state(expect.seat).top.suit;
                                let mut choices = vec![ActionData::Pass];
                                for alone in [false, true] {
                                    for &suit in Suit::all_suits() {
                                        if suit != top_suit {
                                            choices.push(ActionData::BidOther { suit, alone })
                                        }
                                    }
                                }
                                Prompt::new(choices)
                            }
                            ActionType::DealerDiscard | ActionType::Lead | ActionType::Follow => {
                                Prompt::from_iter(
                                    self.game
                                        .round
                                        .player_state(expect.seat)
                                        .sorted_hand()
                                        .into_iter()
                                        .map(|card| ActionData::Card { card }),
                                )
                            }
                        });
                    }
                    return Some(Wait::Action(expect));
                } else {
                    // Robots play automatically without user input.
                    self.game.robot_play(expect);
                    continue;
                }
            }
            return None;
        }
    }

    fn render_frame(&mut self, frame: &mut Frame, wait: &Wait) {
        let areas = Areas::new(frame);
        let p_state = self.game.round.player_state(Seat::South);
        frame.render_widget(self.arena_widget(wait, &p_state), areas.arena);
        frame.render_widget(self.score_widget(&p_state), areas.score);
        frame.render_widget(self.info_widget(wait, &p_state), areas.info);
        frame.render_widget(self.prompt_widget(wait, &p_state), areas.prompt);
    }

    fn handle_events(&mut self, wait: &Wait) -> io::Result<()> {
        match (event::read()?, wait) {
            // The 'q' key always quits.
            (event::Event::Key(k), _) if k.code == KeyCode::Char('q') => self.exit = true,
            (event::Event::Key(k), Wait::Action(expect)) => self.handle_event_for_action(expect, k),
            (event::Event::Key(_), Wait::Round(_)) => {
                // Any key advances to next round.
                self.game.next_round();
            }
            _ => (),
        }
        Ok(())
    }

    fn handle_event_for_action(&mut self, expect: &ExpectAction, key: KeyEvent) {
        match (key.code, self.prompt.as_mut()) {
            (KeyCode::Up | KeyCode::Left | KeyCode::Char('h' | 'k'), Some(prompt)) => {
                prompt.prev();
            }
            (KeyCode::Down | KeyCode::Right | KeyCode::Char('j' | 'l'), Some(prompt)) => {
                prompt.next();
            }
            (KeyCode::Enter | KeyCode::Char(' '), Some(prompt)) => {
                let action = prompt.action(expect);
                if let Err(err) = self.game.round.apply(action) {
                    self.error = Some(err.to_string());
                } else {
                    self.error = None;
                    self.prompt = None;
                }
            }
            _ => (),
        }
    }

    fn arena_widget(&self, wait: &Wait, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let lines = match wait {
            Wait::Deal
            | Wait::Action(ExpectAction {
                action: ActionType::BidTop | ActionType::DealerDiscard,
                ..
            }) => [
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
            Wait::Round(_)
            | Wait::Action(ExpectAction {
                action: ActionType::BidOther | ActionType::Lead,
                ..
            }) => [
                Span::raw("N").into_centered_line(),
                Line::default(),
                Line::default(),
                Span::raw("W          E").into_centered_line(),
                Line::default(),
                Line::default(),
                Span::raw("S").into_centered_line(),
            ],
            Wait::Action(ExpectAction {
                action: ActionType::Follow,
                ..
            }) => [
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
            Wait::Trick(trick) => [
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

    fn info_widget(&self, wait: &Wait, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let mut lines: Vec<Line> = Vec::with_capacity(2);
        if let Some(c) = p_state.contract {
            lines.push(Line::from_iter([
                format!("{} called ", c.maker).into(),
                c.suit.to_span(),
                if c.alone { " alone." } else { "." }.into(),
            ]))
        } else {
            lines.push(format!("{} dealt.", p_state.dealer).into())
        }
        match wait {
            Wait::Deal => (),
            Wait::Action(ExpectAction { seat, action }) => {
                lines.push(format!("{seat} to {action}.").into())
            }
            Wait::Trick(t) => lines.push(format!("{} takes the trick.", t.best().0).into()),
            Wait::Round(Outcome { team, points }) => {
                lines.push(format!("{} win {points} points.", team.to_abbr()).into())
            }
        };
        Paragraph::new(Text::from_iter(lines)).block(Block::bordered())
    }

    fn prompt_widget(&self, wait: &Wait, p_state: &PlayerState<'_>) -> Paragraph<'static> {
        let mut lines = vec![];
        let mut spans = vec![format!("{}'s hand: ", p_state.seat).into()];
        for card in p_state.sorted_hand() {
            let selected = self.prompt.as_ref().and_then(|p| match p.action_data() {
                ActionData::Card { card } => Some(card),
                _ => None,
            });
            let mut card_span = card.to_span();
            if selected.is_some_and(|c| c == card) {
                card_span = card_span.on_dark_gray();
            }
            spans.push(card_span);
            spans.push(" ".into());
        }
        lines.push(Line::from(spans));
        if let Wait::Action(ExpectAction {
            action: ActionType::BidTop | ActionType::BidOther,
            ..
        }) = wait
        {
            let prompt = self.prompt.as_ref().unwrap();
            lines.push("Choose one:".into());
            for (index, choice) in prompt.choices.iter().enumerate() {
                let mut line: Line = match choice {
                    ActionData::Pass => "Pass".into(),
                    ActionData::BidTop { alone: false } => "Pick it up".into(),
                    ActionData::BidTop { alone: true } => "Go alone".into(),
                    ActionData::BidOther { suit, alone } => Line::from_iter([
                        "Call ".into(),
                        suit.to_span(),
                        if *alone { " alone" } else { "" }.into(),
                    ]),
                    _ => unreachable!(),
                };
                if index == prompt.index {
                    line = line.on_dark_gray();
                }
                lines.push(line)
            }
        }
        if let Some(error) = self.error.clone() {
            lines.push(Line::from(error).red().bold())
        }
        if let Some(debug) = self.debug.clone() {
            lines.push(Line::from(debug).blue().bold())
        }
        Paragraph::new(lines)
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
