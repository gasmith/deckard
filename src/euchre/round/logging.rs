//! A round that's capable of logging.

use delegate::delegate;

use crate::euchre::{
    Action, BaseRound, Card, Contract, Event, ExpectAction, Log, LogId, PlayerState, RawLog, Round,
    RoundConfig, RoundError, Seat, Tricks,
};

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
        rand::random::<RoundConfig>().into()
    }

    pub fn restart(&mut self) {
        self.cursor = None;
        self.round = BaseRound::from(self.log.config().clone());
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
}
