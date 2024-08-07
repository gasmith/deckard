//! Round management

use super::{notify, Deck, Dir, Error, Event, Players, Team};

mod bidding;
mod tricks;
use bidding::Bidding;
use tricks::Tricks;

/// The main state machine for the round.
///
/// A new round is initiated by a deal into the `BidTop` state. The players
/// then bid for the top card. If a bid is made, the top card is replaces a
/// card in the dealer's hand, and the round progresses to `Play`.
///
/// Otherwise, the state machine advances to `BidOther`, where players bid for
/// any suit other than that of the top card. The dealer is required to choose
/// a suit, if no other player will.
#[derive(Debug)]
pub enum Round {
    Deal(Bidding),
    BidTop(Bidding),
    BidOther(Bidding),
    Play(Tricks),
    Finish(Outcome),
}

impl Round {
    pub fn new(dealer: Dir, deck: Deck) -> Self {
        Round::Deal(Bidding::new(dealer, deck))
    }

    pub fn run(mut self, players: &Players) -> Result<Outcome, Error> {
        loop {
            self = self.next(players)?;
            if let Round::Finish(outcome) = self {
                return Ok(outcome);
            }
        }
    }

    fn next(self, players: &Players) -> Result<Self, Error> {
        match self {
            Round::Deal(bidding) => {
                bidding.deal(players);
                Ok(Round::BidTop(bidding))
            }
            Round::BidTop(bidding) => match bidding.bid_top(players)? {
                Some(bid) => {
                    notify(players, Event::Bid(bid));
                    bidding.dealer_pick_up_top(players, bid).map(Round::Play)
                }
                None => Ok(Round::BidOther(bidding)),
            },
            Round::BidOther(bidding) => {
                let bid = bidding.bid_other(players)?;
                notify(players, Event::Bid(bid));
                Ok(Round::Play(bidding.into_tricks(bid)))
            }
            Round::Play(mut tricks) => {
                let mut trick = tricks.lead_trick(players)?;
                tricks.follow_trick(players, &mut trick)?;
                notify(players, Event::Trick(trick.clone()));
                tricks.collect_trick(trick);
                Ok(match tricks.outcome() {
                    Some(outcome) => Round::Finish(outcome),
                    None => Round::Play(tricks),
                })
            }
            Round::Finish(_) => Ok(self),
        }
    }
}

/// The outcome of a round.
#[derive(Debug)]
pub struct Outcome {
    pub team: Team,
    pub points: u8,
}

impl Outcome {
    pub fn new(team: Team, points: u8) -> Self {
        Outcome { team, points }
    }
}
