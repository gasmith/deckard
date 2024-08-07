//!

use std::collections::HashMap;

use crate::game::euchre::{
    discard, Bid, Card, Contract, Dir, Error, InvalidPlay, Players, Team, Trick,
};

use super::Outcome;

#[derive(Debug)]
pub struct Tricks {
    hands: HashMap<Dir, Vec<Card>>,
    leader: Dir,
    bid: Bid,
    tricks: HashMap<Dir, Vec<Trick>>,
}

impl Tricks {
    pub fn new(hands: HashMap<Dir, Vec<Card>>, leader: Dir, bid: Bid) -> Self {
        Tricks {
            hands,
            leader,
            bid,
            tricks: HashMap::new(),
        }
    }

    pub fn lead_trick(&mut self, players: &Players) -> Result<Trick, Error> {
        let hand = self.hands.get_mut(&self.leader).unwrap();
        let leader = &players[self.leader];
        loop {
            let card = leader.lead_trick();
            if discard(hand, card) {
                return Ok(Trick::new(self.bid.suit, self.leader, card));
            }
            let invalid = InvalidPlay::CardNotHeld;
            if !leader.invalid_play(invalid) {
                return Err(Error::InvalidPlay(self.leader, invalid));
            }
        }
    }

    pub fn follow_trick(&mut self, players: &Players, trick: &mut Trick) -> Result<(), Error> {
        for dir in self.leader.next_n(3) {
            let hand = self.hands.get_mut(&dir).unwrap();
            let player = &players[dir];
            loop {
                let card = player.follow_trick(trick);
                if !hand.contains(&card) {
                    let invalid = InvalidPlay::CardNotHeld;
                    if !player.invalid_play(invalid) {
                        return Err(Error::InvalidPlay(dir, invalid));
                    }
                } else if !trick.is_following_lead(hand, &card) {
                    let invalid = InvalidPlay::MustFollowLead;
                    if !player.invalid_play(invalid) {
                        return Err(Error::InvalidPlay(dir, invalid));
                    }
                } else {
                    let ok = discard(hand, card);
                    assert!(ok);
                    trick.play(dir, card);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn collect_trick(&mut self, trick: Trick) {
        self.leader = trick.winner;
        self.tricks.entry(trick.winner).or_default().push(trick);
    }

    pub fn outcome(&self) -> Option<Outcome> {
        let mut total_tricks = 0;
        let mut makers_tricks = 0;
        let makers = Team::from(self.bid.dir);
        for (dir, tricks) in &self.tricks {
            total_tricks += tricks.len();
            if Team::from(*dir) == makers {
                makers_tricks += tricks.len();
            }
        }
        if total_tricks - makers_tricks >= 3 {
            // Euchred! No need to keep playing.
            let defenders = makers.other();
            Some(Outcome::new(defenders, 2))
        } else if total_tricks == 5 {
            // All tricks have been played, and the makers were not euchred.
            match (makers_tricks, self.bid.contract) {
                (5, Contract::Alone) => Some(Outcome::new(makers, 4)),
                (5, Contract::Partner) => Some(Outcome::new(makers, 2)),
                _ => Some(Outcome::new(makers, 1)),
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use assert_matches::assert_matches;

    use crate::french::Suit;
    use crate::game::euchre::player::ScriptedPlayer;

    use super::*;

    fn hands_fixture(cards: [(char, [&str; 5]); 4]) -> HashMap<Dir, Vec<Card>> {
        cards
            .map(|(dir, cards)| {
                (
                    Dir::from_char(dir).unwrap(),
                    cards
                        .iter()
                        .map(|s| Card::from_str(s))
                        .collect::<Result<_, _>>()
                        .unwrap(),
                )
            })
            .iter()
            .cloned()
            .collect()
    }

    fn tricks_fixture() -> Tricks {
        Tricks::new(
            hands_fixture([
                ('N', ["JD", "TH", "QD", "9D", "AC"]),
                ('E', ["KH", "JH", "TS", "TD", "QS"]),
                ('S', ["9H", "KD", "9S", "JC", "AD"]),
                ('W', ["JS", "TC", "9C", "QC", "KC"]),
            ]),
            Dir::North,
            Bid {
                dir: Dir::North,
                suit: Suit::Heart,
                contract: Contract::Partner,
            },
        )
    }

    fn players_fixture(players: [(char, ScriptedPlayer); 4]) -> Players {
        Players::new(
            players
                .map(|(dir, p)| (Dir::from_char(dir).unwrap(), p.as_player()))
                .iter()
                .cloned()
                .collect(),
        )
    }

    #[test]
    fn test_lead_trick_card_not_held() {
        let mut tricks = tricks_fixture();
        let players = players_fixture([
            ('N', ScriptedPlayer::default().leads("QH")),
            ('E', ScriptedPlayer::default()),
            ('S', ScriptedPlayer::default()),
            ('W', ScriptedPlayer::default()),
        ]);
        let result = tricks.lead_trick(&players);
        assert!(result.is_err());
        assert_matches!(
            result.err().unwrap(),
            Error::InvalidPlay(Dir::North, InvalidPlay::CardNotHeld)
        )
    }

    #[test]
    fn test_follow_trick_card_not_held() {
        let mut tricks = tricks_fixture();
        let players = players_fixture([
            ('N', ScriptedPlayer::default().leads("JD")),
            ('E', ScriptedPlayer::default().follows("9H")),
            ('S', ScriptedPlayer::default()),
            ('W', ScriptedPlayer::default()),
        ]);
        let mut trick = tricks.lead_trick(&players).unwrap();
        let result = tricks.follow_trick(&players, &mut trick);
        assert_matches!(
            result.err().unwrap(),
            Error::InvalidPlay(Dir::East, InvalidPlay::CardNotHeld)
        )
    }

    #[test]
    fn test_follow_trick_must_follow_lead() {
        let mut tricks = tricks_fixture();
        let players = players_fixture([
            ('N', ScriptedPlayer::default().leads("JD")),
            ('E', ScriptedPlayer::default().follows("TD")),
            ('S', ScriptedPlayer::default()),
            ('W', ScriptedPlayer::default()),
        ]);
        let mut trick = tricks.lead_trick(&players).unwrap();
        let result = tricks.follow_trick(&players, &mut trick);
        assert_matches!(
            result.err().unwrap(),
            Error::InvalidPlay(Dir::East, InvalidPlay::MustFollowLead)
        )
    }

    #[test]
    fn test_full_trick() {
        let mut tricks = tricks_fixture();
        let players = players_fixture([
            ('N', ScriptedPlayer::default().leads("JD")),
            ('E', ScriptedPlayer::default().follows("JH")),
            ('S', ScriptedPlayer::default().follows("9H")),
            ('W', ScriptedPlayer::default().follows("9C")),
        ]);
        let mut trick = tricks.lead_trick(&players).unwrap();
        tricks.follow_trick(&players, &mut trick).unwrap();
        assert_eq!(trick.winner, Dir::East);
        tricks.collect_trick(trick);
        assert_eq!(tricks.leader, Dir::South);
    }
}
