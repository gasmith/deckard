//! A round that's capable of logging.

use delegate::delegate;

use crate::game::euchre::{Action, Card, Contract, Event, ExpectAction, RoundError};

use super::{BaseRound, InitialState, Log, LogId, PlayerState, RawLog, Round, Seat, Tricks};

#[derive(Debug)]
pub struct LoggingRound {
    round: BaseRound,
    log: Log,
    cursor: Option<LogId>,
}
impl From<InitialState> for LoggingRound {
    fn from(initial: InitialState) -> Self {
        Self {
            log: Log::new(initial.clone()),
            round: initial.into(),
            cursor: None,
        }
    }
}
impl From<LoggingRound> for RawLog {
    fn from(value: LoggingRound) -> Self {
        RawLog::from(value.log)
    }
}
impl<'a> From<&'a LoggingRound> for RawLog {
    fn from(value: &'a LoggingRound) -> Self {
        RawLog::from(&value.log)
    }
}

impl Round for LoggingRound {
    delegate! {
        to self.round {
            fn dealer(&self) -> Seat;
            fn top_card(&self) -> Card;
            fn next_action(&self) -> Option<ExpectAction>;
            fn contract(&self) -> Option<Contract>;
            fn tricks(&self) -> &Tricks;
            fn player_state(&self, seat: Seat) -> PlayerState<'_>;
            fn pop_event(&mut self) -> Option<Event>;
        }
    }

    fn apply_action(&mut self, action: Action) -> Result<(), RoundError> {
        self.round.apply_action(action)?;
        self.cursor = Some(self.log.insert(self.cursor, action));
        Ok(())
    }
}

impl LoggingRound {
    pub fn cursor(&self) -> Option<LogId> {
        self.cursor
    }

    pub fn log(&self) -> &Log {
        &self.log
    }

    pub fn random() -> Self {
        rand::random::<InitialState>().into()
    }

    pub fn random_with_dealer(dealer: Seat) -> Self {
        InitialState::random_with_dealer(dealer).into()
    }

    pub fn restart(&mut self) {
        self.cursor = None;
        self.round = BaseRound::from(self.log.initial().clone());
    }

    pub fn seek(&mut self, id: Option<LogId>) -> Result<(), RoundError> {
        self.restart();
        if let Some(id) = id {
            for (id, action) in self.log.backtrace(id)? {
                self.round.apply_action(action)?;
                self.cursor = Some(id);
            }
        }
        Ok(())
    }

    pub fn backtrace(&self) -> Vec<(LogId, Action)> {
        self.cursor
            .map(|id| self.log.backtrace(id).expect("cursor valid"))
            .unwrap_or_default()
    }
}
