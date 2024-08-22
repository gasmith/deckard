use std::fs::File;
use std::io::{self, stdout, Stdout};

use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, ExecutableCommand};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

mod action;
mod arena;
mod hand;
mod history;
mod info;
mod scoreboard;
use self::action::{ActionChoice, ActionChoiceState};
use self::arena::Arena;
use self::hand::{Hand, HandState};
use self::history::{History, HistoryState};
use self::info::Info;
use self::scoreboard::Scoreboard;

use super::{
    Action, ActionData, ActionType, Event, ExpectAction, Game, LogId, LoggingRound, Player, RawLog,
    Robot, Round, Seat,
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

struct Areas {
    arena: Rect,
    score: Rect,
    info: Rect,
    hand: Rect,
    action: Rect,
    message: Rect,
    history: Rect,
}
impl Areas {
    fn new(frame: &Frame, mode: &Mode) -> Self {
        let [game, history] = Layout::new(
            Direction::Horizontal,
            [Constraint::Length(40), Constraint::Min(20)],
        )
        .areas(frame.area());
        let action_size = if let Mode::ActionChoice(choice, _) = mode {
            choice.len() as u16
        } else {
            0
        };
        let [arena_score_info, hand, action, message] = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(9),           // arena & score & info
                Constraint::Length(1),           // hand
                Constraint::Length(action_size), // optional action
                Constraint::Min(2),              // optional messages
            ],
        )
        .areas(game);
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
            hand,
            action,
            message,
            history,
        }
    }
}

#[derive(Debug)]
enum Mode {
    Event(Event),
    Hand(Hand, HandState),
    ActionChoice(ActionChoice, ActionChoiceState),
    History(History, HistoryState),
}

impl Mode {
    fn event(event: Event) -> Self {
        Self::Event(event)
    }
    fn hand(hand: Hand) -> Self {
        Self::Hand(hand, HandState::default().with_selected(Some(0)))
    }
    fn action_choice(choice: ActionChoice) -> Self {
        Self::ActionChoice(choice, ActionChoiceState::default().with_selected(Some(0)))
    }
    fn history(history: History) -> Self {
        Self::History(history, HistoryState::default().with_selected(Some(0)))
    }
}

const HUMAN_SEAT: Seat = Seat::South;

pub struct Tui {
    mode: Mode,
    game: Game<LoggingRound>,
    robot: Robot,
    error: Option<String>,
    debug: Option<String>,
    exit: bool,
}

impl Default for Tui {
    fn default() -> Self {
        let mut game: Game<LoggingRound> = Game::default();
        let event = game.round_mut().pop_event().expect("deal");
        Self {
            mode: Mode::Event(event),
            game,
            robot: Robot::default(),
            error: None,
            debug: None,
            exit: false,
        }
    }
}

impl Tui {
    pub fn run(mut self, mut terminal: Term) -> anyhow::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let areas = Areas::new(frame, &self.mode);
        let round = self.game.round();
        frame.render_widget(Arena::new(&self.mode, round), areas.arena);
        frame.render_widget(Scoreboard::new(&self.game), areas.score);
        frame.render_widget(Info::new(&self.mode, &self.game), areas.info);
        if let Mode::Hand(hand, state) = &mut self.mode {
            frame.render_stateful_widget(hand.clone(), areas.hand, state);
        } else {
            let seat = HUMAN_SEAT;
            let hand = round.player_state(seat).sorted_hand();
            frame.render_widget(Hand::new(seat, hand), areas.hand);
        }
        if let Mode::ActionChoice(choice, state) = &mut self.mode {
            frame.render_stateful_widget(choice.clone(), areas.action, state);
        }
        if let Mode::History(history, state) = &mut self.mode {
            frame.render_stateful_widget(history.clone(), areas.history, state);
        }
        let mut lines = vec![];
        if let Some(error) = self.error.clone() {
            lines.push(Line::from(error).red().bold())
        }
        if let Some(debug) = self.debug.clone() {
            lines.push(Line::from(debug).blue().bold())
        }
        frame.render_widget(Paragraph::new(lines), areas.message);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        let event::Event::Key(key) = event::read()? else {
            return Ok(());
        };

        // Output messages only persist for one refresh cycle.
        self.error = None;
        self.debug = None;

        match (&mut self.mode, key.code) {
            // Quit!
            (_, KeyCode::Char('q')) => self.exit = true,

            // TODO: Make this less of a hack, prompt for filename, etc.
            (_, KeyCode::Char('s')) => {
                let file = File::create("euchre.json").expect("open euchre.json");
                let log = RawLog::from(self.game.round());
                serde_json::to_writer(file, &log).expect("write to euchre.json");
                self.debug = Some("Wrote to euchre.json".into());
            }

            // Toggle history mode
            (Mode::History(_, _), KeyCode::Char('!')) => self.game_step(),
            (_, KeyCode::Char('!')) => self.enter_history_mode(),

            // Events are informational, any key steps forward.
            (Mode::Event(_), _) => self.game_step(),

            // Hand management.
            (Mode::Hand(hand, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                if let Some(action) = self
                    .game
                    .round()
                    .next_action()
                    .zip(hand.selected(state))
                    .map(|(expect, card)| expect.with_data(ActionData::Card { card }))
                {
                    self.apply_action(action);
                }
            }
            (Mode::Hand(_, s), KeyCode::Left | KeyCode::Char('h')) => s.select_previous(),
            (Mode::Hand(_, s), KeyCode::Right | KeyCode::Char('l')) => s.select_next(),

            // Action choices.
            (Mode::ActionChoice(choice, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                if let Some(action) = self
                    .game
                    .round()
                    .next_action()
                    .zip(choice.selected(state))
                    .map(|(expect, data)| expect.with_data(data))
                {
                    self.apply_action(action);
                }
            }
            (Mode::ActionChoice(_, s), KeyCode::Up | KeyCode::Char('k')) => s.select_previous(),
            (Mode::ActionChoice(_, s), KeyCode::Down | KeyCode::Char('j')) => s.select_next(),

            // History browser.
            (Mode::History(history, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                if let Some(id) = history.selected(state) {
                    self.seek_round_history(id)
                }
            }
            (Mode::History(_, s), KeyCode::Up | KeyCode::Char('k')) => s.select_previous(),
            (Mode::History(_, s), KeyCode::Down | KeyCode::Char('j')) => s.select_next(),

            _ => (),
        }

        Ok(())
    }

    /// Advances the state of the game until an event occurs, or the game is
    /// blocked waiting on a non-robot player's action. Internally takes care
    /// of advancing to the next round, if the game is not over.
    fn game_step(&mut self) {
        loop {
            // Drain round events before checking for end-of-round.
            if let Some(event) = self.game.round_mut().pop_event() {
                self.mode = Mode::event(event);
                break;
            }

            // Check for end of round & end of game.
            if self.game.round().outcome().is_some() {
                self.game.next_round();
                if let Some(outcome) = self.game.outcome() {
                    self.mode = Mode::event(Event::Game(outcome));
                    break;
                } else {
                    continue;
                }
            }

            // Handle round actions.
            if let Some(expect @ ExpectAction { seat, action }) = self.game.round().next_action() {
                if seat == HUMAN_SEAT {
                    // Switch mode for human input.
                    self.mode = match action {
                        ActionType::BidTop => {
                            let top_suit = self.game.round().top_card().suit;
                            Mode::action_choice(ActionChoice::bid_top(top_suit))
                        }
                        ActionType::BidOther => {
                            let top_suit = self.game.round().top_card().suit;
                            Mode::action_choice(ActionChoice::bid_other(top_suit))
                        }
                        ActionType::DealerDiscard | ActionType::Lead | ActionType::Follow => {
                            let cards = self.game.round().player_state(seat).sorted_hand();
                            Mode::hand(Hand::new(seat, cards))
                        }
                    };
                    break;
                } else {
                    // Play as the robot.
                    let round = self.game.round_mut();
                    let state = round.player_state(seat);
                    let data = self.robot.take_action(state, action);
                    let action = expect.with_data(data);
                    round.apply_action(action).expect("robots don't err");
                    continue;
                }
            }
        }
    }

    /// Applies the specified action to the game and updates the mode.
    fn apply_action(&mut self, action: Action) {
        if let Err(err) = self.game.round_mut().apply_action(action) {
            self.error = Some(err.to_string());
        } else {
            self.game_step();
        }
    }

    /// Enters history browser mode.
    fn enter_history_mode(&mut self) {
        self.mode = Mode::history(History::new(self.game.round().backtrace()))
    }

    /// Seeks to a particular point in round history.
    fn seek_round_history(&mut self, id: LogId) {
        if let Err(e) = self.game.round_mut().seek(id) {
            self.error = Some(e.to_string())
        } else {
            while self.game.round_mut().pop_event().is_some() {}
            self.game_step()
        }
    }
}
