//! The game of euchre.
//!
//! Todo:
//!
//!  - Feature flag for no-trump
//!  - Feature flag for stick-the-dealer

mod action;
mod card;
mod error;
mod game;
mod player;
mod round;
mod seat;
mod trick;
mod tui;
pub use action::{Action, ActionData, ActionType, ExpectAction};
pub use card::{Card, Deck, Rank, Suit};
pub use error::{PlayerError, RoundError};
pub use game::{Game, GameOutcome};
pub use player::{Console, Player, Robot};
pub use round::{BaseRound, LogId, LoggingRound, PlayerState, RawLog, Round, RoundOutcome};
pub use seat::{Seat, Team};
pub use trick::Trick;
pub use tui::{tui_init, tui_restore, Tui};

#[derive(Debug, Clone)]
pub enum Event {
    Deal(Seat, Card),
    Bid(Contract),
    Trick(Trick),
    Round(RoundOutcome),
    Game(GameOutcome),
}

#[derive(Debug, Clone, Copy)]
pub struct Contract {
    pub maker: Seat,
    pub suit: Suit,
    pub alone: bool,
}

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
