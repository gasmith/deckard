//! The game of euchre.

mod action;
mod card;
mod error;
mod game;
mod player;
mod round;
mod seat;
mod trick;
mod tui;
use self::action::{Action, ActionData, ActionType, ExpectAction};
use self::card::{Card, Deck, Rank, Suit};
use self::error::{PlayerError, RoundError};
use self::game::Game;
use self::player::{Console, Player, Robot};
use self::round::{
    BaseRound, Contract, Log, LogId, LoggingRound, PlayerState, RawLog, Round, RoundConfig,
    RoundOutcome, Tricks,
};
use self::seat::{Seat, Team};
use self::trick::Trick;
use self::tui::{tui_init, tui_restore, Tui};

/// An event that occurs during the game.
#[derive(Debug, Clone)]
enum Event {
    /// The dealer dealt and revealed the top card.
    Deal(Seat, Card),
    /// A player declared a contract.
    Call(Contract),
    /// The trick is over.
    Trick(Trick),
    /// The round is over.
    Round(RoundOutcome),
    /// The game is over.
    Game(Team),
}

/// Runs the game with a simple command-line interface.
pub fn cli_main() {
    let console = Console::default().into_player();
    let robot = Robot::default().into_player();

    let mut round = LoggingRound::random();
    for my_seat in [Seat::South] {
        round.restart();
        println!("You are {my_seat}");
        loop {
            while let Some(event) = round.pop_event() {
                console.notify(round.player_state(my_seat), &event);
            }
            let Some(expect) = round.next_action() else {
                break;
            };
            let player = if expect.seat == my_seat {
                &console
            } else {
                &robot
            };
            let player_state = round.player_state(expect.seat);
            let data = player.take_action(player_state, expect.action);
            let action = expect.with_data(data);
            match round.apply_action(action) {
                Err(RoundError::Player(err)) if player.handle_error(err.clone()) => continue,
                Err(err) => panic!("Fatal: {}", err),
                _ => (),
            }
        }
    }
    let log = RawLog::from(round);
    serde_json::to_writer(std::io::stderr(), &log).unwrap();
}

/// Runs the game in a rich terminal UI.
pub fn tui_main() {
    let tui = Tui::default();
    let terminal = tui_init().unwrap();
    tui.run(terminal).unwrap();
    tui_restore().unwrap();
}
