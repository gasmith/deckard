//! Mechanics of the deal & bid.

use std::collections::HashMap;

use super::tricks::Tricks;
use crate::game::euchre::{discard, Card, Contract, Deck, Dir, Error, InvalidPlay, Players};

#[derive(Debug)]
pub struct Bidding {
    hands: HashMap<Dir, Vec<Card>>,
    dealer: Dir,
    top: Card,
}

impl Bidding {
    pub fn new(dealer: Dir, mut deck: Deck) -> Self {
        assert_eq!(24, deck.len());
        let hands = dealer
            .next_n(4)
            .into_iter()
            .map(|dir| (dir, deck.take(5)))
            .collect();
        let top = deck.take(1)[0];
        println!("Dealer: {dealer:?}");
        println!("Top: {top}");
        Bidding { hands, dealer, top }
    }

    pub fn deal(&self, players: &Players) {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            let hand = self.hands.get(&dir).expect("hands");
            player.deal(self.dealer, hand.clone(), self.top);
        }
    }

    pub fn bid_top(&self, players: &Players) -> Result<Option<Contract>, Error> {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            if let Some(alone) = player.bid_top() {
                let bid = Contract {
                    maker: dir,
                    suit: self.top.suit,
                    alone,
                };
                return Ok(Some(bid));
            }
        }
        Ok(None)
    }

    pub fn dealer_pick_up_top(
        mut self,
        players: &Players,
        contract: Contract,
    ) -> Result<Tricks, Error> {
        let hand = self.hands.get_mut(&self.dealer).unwrap();
        hand.push(self.top);
        let dealer = &players[self.dealer];
        loop {
            let card = dealer.pick_up_top(self.top);
            if discard(hand, card) {
                return Ok(self.into_tricks(contract));
            }
            // The dealer attempted to discard a card they do not hold.
            let invalid = InvalidPlay::CardNotHeld;
            if !dealer.invalid_play(invalid) {
                return Err(Error::InvalidPlay(self.dealer, invalid));
            }
        }
    }

    pub fn bid_other(&self, players: &Players) -> Result<Contract, Error> {
        for dir in self.dealer.next_n(4) {
            let player = &players[dir];
            loop {
                match player.bid_other() {
                    Some((suit, alone)) => {
                        if suit == self.top.suit {
                            let invalid = InvalidPlay::CannotBidTopSuit;
                            if !player.invalid_play(invalid) {
                                return Err(Error::InvalidPlay(dir, invalid));
                            }
                        }
                        return Ok(Contract {
                            maker: dir,
                            suit,
                            alone,
                        });
                    }
                    None if dir == self.dealer => {
                        let invalid = InvalidPlay::DealerMustBid;
                        if !player.invalid_play(invalid) {
                            return Err(Error::InvalidPlay(dir, invalid));
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }
        unreachable!();
    }

    pub fn into_tricks(self, contract: Contract) -> Tricks {
        Tricks::new(self.hands, self.dealer, contract)
    }
}
