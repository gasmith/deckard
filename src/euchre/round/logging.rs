//! A round that maintains a log of actions taken.

use delegate::delegate;

use crate::euchre::{
    Action, BaseRound, Card, Contract, Event, ExpectAction, Log, LogId, PlayerState, RawLog, Round,
    RoundConfig, RoundError, Seat, Tricks,
};

/// A [`Round`] implementation that maintains a [`Log`] of all actions taken.
#[derive(Debug)]
pub struct LoggingRound {
    round: BaseRound,
    log: Log,
    cursor: Option<LogId>,
}
impl From<RoundConfig> for LoggingRound {
    fn from(config: RoundConfig) -> Self {
        Self {
            log: Log::new(config.clone()),
            round: config.into(),
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
impl From<Log> for LoggingRound {
    fn from(log: Log) -> Self {
        let round = log.config().clone().into();
        Self {
            log,
            round,
            cursor: None,
        }
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
    /// Creates a new random [`LoggingRound`].
    pub fn random() -> Self {
        rand::random::<RoundConfig>().into()
    }

    /// Returns a cursor pointing to the last action taken.
    pub fn cursor(&self) -> Option<LogId> {
        self.cursor
    }

    /// Returns an immutable reference to the log.
    pub fn log(&self) -> &Log {
        &self.log
    }

    /// Restarts the round.
    pub fn restart(&mut self) {
        self.cursor = None;
        self.round = BaseRound::from(self.log.config().clone());
    }

    /// Seeks to the specified action in the log.
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
}
