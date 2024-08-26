//! The game of euchre.

use std::collections::HashMap;

use super::{Round, RoundConfig, Team};

#[derive(Debug, Clone)]
pub struct GameOutcome {
    pub team: Team,
    #[allow(dead_code)]
    pub score: HashMap<Team, u8>,
}

pub struct Game<R> {
    round: R,
    score: HashMap<Team, u8>,
}

impl<R> Default for Game<R>
where
    R: Round + From<RoundConfig>,
{
    fn default() -> Self {
        Self {
            round: RoundConfig::random().into(),
            score: [(Team::NorthSouth, 0), (Team::EastWest, 0)]
                .iter()
                .copied()
                .collect(),
        }
    }
}

impl<R> Game<R>
where
    R: Round,
{
    pub fn round(&self) -> &R {
        &self.round
    }

    pub fn round_mut(&mut self) -> &mut R {
        &mut self.round
    }

    pub fn outcome(&self) -> Option<GameOutcome> {
        for (&team, &points) in &self.score {
            if points >= 10 {
                return Some(GameOutcome {
                    team,
                    score: self.score.clone(),
                });
            }
        }
        None
    }

    pub fn score(&self, team: Team) -> u8 {
        self.score.get(&team).copied().unwrap_or_default()
    }
}

impl<R> Game<R>
where
    R: Round + From<RoundConfig>,
{
    pub fn next_round(&mut self) {
        let outcome = self.round.outcome().expect("round must be over");
        let score = self.score.entry(outcome.team).or_default();
        *score += outcome.points;
        let dealer = self.round.dealer().next();
        self.round = RoundConfig::random_with_dealer(dealer).into();
    }
}
