//! Game management.
//!
//! A game consists of a sequence of [`Round`]s, by which [`Team`]s score points. A team wins the
//! game by scoring ten or more points.

use std::collections::HashMap;

use super::{Round, RoundConfig, Team};

/// A game of euchre.
pub struct Game<R> {
    /// The current round.
    round: R,
    /// The current scores.
    score: HashMap<Team, u8>,
    /// The target score.
    target_score: u8,
}

impl<R> Default for Game<R>
where
    R: Round + From<RoundConfig>,
{
    fn default() -> Self {
        let round = R::from(RoundConfig::random());
        Self::from(round)
    }
}

impl<R> From<R> for Game<R> {
    fn from(round: R) -> Self {
        Self {
            round,
            score: [(Team::NorthSouth, 0), (Team::EastWest, 0)]
                .iter()
                .copied()
                .collect(),
            target_score: 10,
        }
    }
}

impl<R> Game<R>
where
    R: Round,
{
    /// Sets the target score.
    pub fn with_target_score(mut self, score: u8) -> Self {
        self.target_score = score;
        self
    }

    /// Returns an immutable reference to the current round.
    pub fn round(&self) -> &R {
        &self.round
    }

    /// Returns an mutable reference to the current round.
    pub fn round_mut(&mut self) -> &mut R {
        &mut self.round
    }

    /// Returns the winning team, if the game is over.
    pub fn winner(&self) -> Option<Team> {
        for (&team, &points) in &self.score {
            if points >= self.target_score {
                return Some(team);
            }
        }
        None
    }

    /// Returns the outcome of the game, if it is over.
    pub fn score(&self, team: Team) -> u8 {
        self.score.get(&team).copied().unwrap_or_default()
    }
}

impl<R> Game<R>
where
    R: Round + From<RoundConfig>,
{
    /// Updates the score from the outcome of the current round, and begins a new round. It is the
    /// caller's responsibility to ensure that the current round is finished.
    pub fn next_round(&mut self) {
        let outcome = self.round.outcome().expect("round must be over");
        let score = self.score.entry(outcome.team).or_default();
        *score += outcome.points;
        let dealer = self.round.dealer().next();
        self.round = RoundConfig::random_with_dealer(dealer).into();
    }
}
