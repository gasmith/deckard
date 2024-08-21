//! The game of euchre.
//!
//! Todo:
//!
//!  - Feature flag for no-trump
//!  - Feature flag for stick-the-dealer

mod action;
mod card;
mod error;
mod player;
mod round;
mod seat;
mod trick;
mod tui;
pub use action::{Action, ActionData, ActionType, ExpectAction};
pub use card::{Card, Deck, Rank, Suit};
pub use error::{PlayerError, RoundError};
pub use player::{Console, Player, Robot};
pub use round::{LogId, LoggingRound, Outcome, PlayerState, RawLog, Round};
pub use seat::{Seat, Team};
pub use trick::Trick;
pub use tui::{Tui, tui_init, tui_restore};

/*
/// A euchre game consists of a set of players.
struct Euchre {
    players: Players,
    next_dealer: Seat,
    points: HashMap<Team, u8>,
}

impl Euchre {
    pub fn new(players: Players) -> Self {
        Self {
            players,
            next_dealer: Seat::North,
            points: HashMap::from([(Team::NorthSouth, 0), (Team::EastWest, 0)]),
        }
    }

    fn run_round<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<Outcome, Error> {
        let round = Round::new(self.next_dealer, rng);
        let outcome = round.run(&self.players)?;
        let points = self.points.get_mut(&outcome.team).expect("init points");
        notify(&self.players, Event::Round(outcome.clone()));
        *points += outcome.points;
        self.next_dealer = self.next_dealer.next();
        println!("Standings: {:?}", self.points);
        Ok(outcome)
    }

    #[allow(dead_code)]
    fn winner(&self) -> Option<Team> {
        for (team, points) in &self.points {
            if *points >= 10 {
                return Some(*team);
            }
        }
        None
    }
}
*/

#[derive(Debug, Clone)]
pub enum Event {
    Deal(Seat, Card),
    Bid(Contract),
    Trick(Trick),
    Round(Outcome),
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
            let Some(expect) = round.next() else {
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
            match round.apply(action) {
                Err(RoundError::Player(err)) if player.handle_error(err.clone()) => continue,
                Err(err) => panic!("Fatal: {}", err),
                _ => (),
            }
        }
    }
    let log = RawLog::from(round);
    serde_json::to_writer(std::io::stderr(), &log).unwrap();
}
