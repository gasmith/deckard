//! Rich terminal UI.

use std::convert::TryFrom;
use std::fs::File;
use std::io::{self, stdout, Stdout};
use std::path::Path;

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

use super::action::ActionData;
use super::{
    Action, ActionType, Event, ExpectAction, Game, LogId, LoggingRound, Player, RawLog, Robot,
    Round, Seat,
};

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Initializes the terminal for the TUI.
pub fn tui_init() -> io::Result<Term> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    Ok(terminal)
}

/// Restores the original terminal mode.
pub fn tui_restore() -> io::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

/// Helper struct to keep track of UI areas in the layout.
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
    /// Performs layout math to figure out the render areas.
    fn new(frame: &Frame, mode: &Mode) -> Self {
        let [game, history] = Layout::new(
            Direction::Horizontal,
            [Constraint::Length(40), Constraint::Min(20)],
        )
        .areas(frame.area());
        let action_size = if let Mode::ActionChoice(choice, _) = mode {
            u16::try_from(choice.len()).expect("less than 2^16")
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

/// Modal interface state.
#[derive(Debug)]
enum Mode {
    /// Display an event to the user.
    Event(Event),
    /// Prompt the user to select a card from a player's hand.
    Hand(Hand, HandState),
    /// Prompt the user to select an action for the player.
    ActionChoice(ActionChoice, ActionChoiceState),
    /// Show the interactive history explorer.
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
    fn history(history: History, selected: Option<usize>) -> Self {
        Self::History(history, HistoryState::default().with_selected(selected))
    }
}

/// The human player's seat at the table.
const HUMAN_SEAT: Seat = Seat::South;

/// Terminal UI state.
pub struct Tui {
    /// The current mode.
    mode: Mode,
    /// The game being played.
    game: Game<LoggingRound>,
    /// The robot implementation.
    robot: Robot,
    /// Whether to auto-play as robots.
    robot_autoplay: bool,
    /// An error message to display to the user.
    error: Option<String>,
    /// A debug message to display to the user.
    debug: Option<String>,
    /// Set to true ot exit the main loop.
    exit: bool,
}

impl Default for Tui {
    fn default() -> Self {
        Game::default().into()
    }
}
impl From<Game<LoggingRound>> for Tui {
    fn from(mut game: Game<LoggingRound>) -> Self {
        let event = game.round_mut().pop_event().expect("deal");
        Self {
            mode: Mode::Event(event),
            game,
            robot: Robot::default(),
            robot_autoplay: true,
            error: None,
            debug: None,
            exit: false,
        }
    }
}

impl Tui {
    /// Loads a saved round from a file.
    pub fn from_round_file(log_path: &Path) -> anyhow::Result<Self> {
        let log = RawLog::from_json_file(log_path)?.into_log();
        let round = LoggingRound::from(log);
        let game = Game::from(round).with_target_score(1);
        Ok(game.into())
    }

    /// Runs the terminal UI until the user exits.
    pub fn run(mut self, mut terminal: Term) -> anyhow::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    // Top-level frame renderer.
    fn render_frame(&mut self, frame: &mut Frame) {
        let areas = Areas::new(frame, &self.mode);
        let round = self.game.round();
        frame.render_widget(Arena::new(&self.mode, round), areas.arena);
        frame.render_widget(Scoreboard::new(&self.game), areas.score);
        frame.render_widget(Info::new(&self.mode, &self.game), areas.info);
        match &mut self.mode {
            Mode::Hand(hand, state) => {
                frame.render_stateful_widget(hand.clone(), areas.hand, state);
            }
            Mode::ActionChoice(_, _) => {
                let seat = round.next_action().map_or(HUMAN_SEAT, |e| e.seat);
                self.render_hand_for_seat(seat, frame, areas.hand);
            }
            Mode::History(_, _) => self.render_current_hand(frame, areas.hand),
            Mode::Event(_) => (),
        }
        if let Mode::ActionChoice(choice, state) = &mut self.mode {
            frame.render_stateful_widget(choice.clone(), areas.action, state);
        }
        if let Mode::History(history, state) = &mut self.mode {
            frame.render_stateful_widget(history.clone(), areas.history, state);
        }
        let mut lines = vec![];
        if let Some(error) = self.error.clone() {
            lines.push(Line::from(error).red().bold());
        }
        if let Some(debug) = self.debug.clone() {
            lines.push(Line::from(debug).blue().bold());
        }
        frame.render_widget(Paragraph::new(lines), areas.message);
    }

    /// Renders the current player's hand.
    fn render_current_hand(&self, frame: &mut Frame, area: Rect) {
        if let Some(seat) = self.game.round().next_action().map(|expect| expect.seat) {
            self.render_hand_for_seat(seat, frame, area);
        }
    }

    /// Renders the hand for the specified player.
    fn render_hand_for_seat(&self, seat: Seat, frame: &mut Frame, area: Rect) {
        let hand = self.game.round().player_state(seat).sorted_hand();
        frame.render_widget(Hand::new(seat, hand), area);
    }

    /// Top-level event handler.
    fn handle_events(&mut self) -> io::Result<()> {
        let event::Event::Key(key) = event::read()? else {
            return Ok(());
        };

        // Output messages only persist for one refresh cycle.
        self.error = None;
        self.debug = None;

        #[allow(clippy::match_same_arms)]
        match (&mut self.mode, key.code) {
            // Quit, or exit history
            (Mode::History(_, _), KeyCode::Char('!' | 'q')) => self.game_step(),
            (_, KeyCode::Char('q')) => self.exit = true,

            // End of game
            (Mode::Event(Event::Game(_)), _) => (),

            // Enter history mode
            (_, KeyCode::Char('!')) => self.enter_history_mode(),

            // Save the game log
            (_, KeyCode::Char('s')) => self.save_round(),

            // What would the robot do?
            (Mode::Hand(_, _) | Mode::ActionChoice(_, _), KeyCode::Char('?')) => self.ask_robot(),

            // Toggle robot autoplay
            (_, KeyCode::Char('@')) => self.toggle_robot_autoplay(),

            // Event acknowledgement
            (Mode::Event(Event::Round(_)), _) => self.next_round(),
            (Mode::Event(_), _) => self.game_step(),

            // Hand management
            (Mode::Hand(hand, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                let expect = self.game.round().next_action();
                if let Some(action) = hand.action(state, expect) {
                    self.apply_action(action);
                }
            }
            (Mode::Hand(_, s), KeyCode::Left | KeyCode::Char('h')) => s.select_previous(),
            (Mode::Hand(_, s), KeyCode::Right | KeyCode::Char('l')) => s.select_next(),

            // Action choices
            (Mode::ActionChoice(choice, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                let expect = self.game.round().next_action();
                if let Some(action) = choice.action(state, expect) {
                    self.apply_action(action);
                }
            }
            (Mode::ActionChoice(_, s), KeyCode::Up | KeyCode::Char('k')) => s.select_previous(),
            (Mode::ActionChoice(_, s), KeyCode::Down | KeyCode::Char('j')) => s.select_next(),

            // History browser
            (Mode::History(history, state), KeyCode::Enter | KeyCode::Char(' ')) => {
                if let Some(id) = history.selected(state) {
                    self.seek_round_history(id);
                    self.game_step();
                }
            }
            (Mode::History(history, state), KeyCode::Up | KeyCode::Char('k')) => {
                state.select_previous();
                if let Some(id) = history.selected(state) {
                    self.seek_round_history(id);
                }
            }
            (Mode::History(history, state), KeyCode::Down | KeyCode::Char('j')) => {
                state.select_next();
                if let Some(id) = history.selected(state) {
                    self.seek_round_history(id);
                }
            }

            _ => (),
        }

        Ok(())
    }

    /// Starts the next round of the game, and checks to see if the game is over.
    fn next_round(&mut self) {
        self.game.next_round();
        if let Some(team) = self.game.winner() {
            self.mode = Mode::event(Event::Game(team));
        } else {
            self.game_step();
        }
    }

    /// Advances the state of the game until an event occurs, or the game is
    /// blocked waiting on a non-robot player's action. Internally takes care
    /// of advancing to the next round, if the game is not over.
    fn game_step(&mut self) {
        loop {
            // Drain events.
            if let Some(event) = self.game.round_mut().pop_event() {
                self.mode = Mode::event(event);
                break;
            }

            // We may have missed the end-of-round event, because we dropped events in
            // `seek_round_history`. Generate a synthetic event.
            if let Some(outcome) = self.game.round().outcome() {
                self.mode = Mode::event(Event::Round(outcome));
                break;
            }

            // Handle round actions.
            if let Some(expect) = self.game.round().next_action() {
                if expect.seat == HUMAN_SEAT || !self.robot_autoplay {
                    self.await_user_action(expect);
                    break;
                }
                self.play_as_robot(expect);
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

    /// Updates the UI mode to await user input for an action.
    fn await_user_action(&mut self, expect: ExpectAction) {
        self.mode = match expect.action {
            ActionType::BidTop => {
                let top_suit = self.game.round().top_card().suit;
                Mode::action_choice(ActionChoice::bid_top(top_suit))
            }
            ActionType::BidOther => {
                let top_suit = self.game.round().top_card().suit;
                Mode::action_choice(ActionChoice::bid_other(top_suit))
            }
            ActionType::DealerDiscard | ActionType::Lead | ActionType::Follow => {
                let cards = self.game.round().player_state(expect.seat).sorted_hand();
                Mode::hand(Hand::new(expect.seat, cards))
            }
        };
    }

    /// Asks what the robot would do, displaying the result as a debug message.
    fn ask_robot(&mut self) {
        let round = self.game.round();
        if let Some(expect) = round.next_action() {
            let state = round.player_state(expect.seat);
            let data = self.robot.take_action(state, expect.action);
            let suggest = match data {
                ActionData::Pass => "Pass".into(),
                ActionData::Call { suit, alone: false } => format!("Call {suit}"),
                ActionData::Call { suit, alone: true } => format!("Call {suit} alone"),
                ActionData::Card { card } => card.to_string(),
            };
            self.debug = Some(format!("Robot suggests: {suggest}"));
        }
    }

    /// Toggle robot autoplay.
    fn toggle_robot_autoplay(&mut self) {
        self.robot_autoplay = !self.robot_autoplay;

        // If we're currently waiting for the user to take action on behalf of a robot player,
        // advance the state machine automatically.
        if self.robot_autoplay && matches!(self.mode, Mode::ActionChoice(_, _) | Mode::Hand(_, _)) {
            self.game_step();
        }

        self.debug = Some(format!(
            "Robot autoplay {}",
            if self.robot_autoplay {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }

    /// Uses the robot to resolve the next action.
    fn play_as_robot(&mut self, expect: ExpectAction) {
        let round = self.game.round_mut();
        let state = round.player_state(expect.seat);
        let data = self.robot.take_action(state, expect.action);
        let action = expect.with_data(data);
        round.apply_action(action).expect("robots don't err");
    }

    /// Enters history browser mode.
    fn enter_history_mode(&mut self) {
        let round = self.game.round();
        let cursor = round.cursor();
        let history = History::new(cursor, round.log());
        let index = history.cursor_position();
        self.mode = Mode::history(history, index);
    }

    /// Seeks to a particular point in round history.
    fn seek_round_history(&mut self, id: Option<LogId>) {
        if let Err(e) = self.game.round_mut().seek(id) {
            self.error = Some(e.to_string());
        } else {
            // Drop events.
            while self.game.round_mut().pop_event().is_some() {}
        }
    }

    /// Saves the round to a file.
    fn save_round(&mut self) {
        // TODO: Make this less of a hack... add an input for filename, etc.
        if let Err(e) = self.try_save_round() {
            self.error = Some(format!("Failed to write euchre.json: {e}"));
        } else {
            self.debug = Some("Wrote to euchre.json".into());
        }
    }

    /// Tries to save the round to a file, or returns an error.
    fn try_save_round(&self) -> Result<(), anyhow::Error> {
        let file = File::create("euchre.json")?;
        let log = RawLog::from(self.game.round());
        serde_json::to_writer(file, &log)?;
        Ok(())
    }
}
