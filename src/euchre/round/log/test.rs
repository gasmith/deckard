use std::str::FromStr;

use maplit::hashmap;

use crate::euchre::{ActionData, ActionType, Card, Seat, Suit};

use super::*;

fn card<S: AsRef<str>>(s: S) -> Card {
    Card::from_str(s.as_ref()).unwrap()
}

fn hand(cards: [&str; 5]) -> Vec<Card> {
    cards.iter().map(card).collect()
}

fn config_fixture() -> RoundConfig {
    RoundConfig {
        dealer: Seat::North,
        hands: hashmap! {
            Seat::North => hand(["ad", "qs", "jh", "th", "9h"]),
            Seat::East => hand(["jc", "kd", "ks", "kh", "qh"]),
            Seat::South => hand(["ac", "kc", "qc", "qd", "td"]),
            Seat::West => hand(["tc", "js", "ts", "9s", "ah"]),
        },
        top: Card::from_str("jd").unwrap(),
    }
}

fn raw_log_fixture() -> RawLog {
    let config = config_fixture();
    let actions = vec![
        ActionNode {
            id: 0,
            parent: None,
            action: Action::new(Seat::East, ActionType::BidTop, ActionData::Pass),
        },
        ActionNode {
            id: 1,
            parent: Some(0),
            action: Action::new(
                Seat::South,
                ActionType::BidTop,
                ActionData::Call {
                    suit: Suit::Diamond,
                    alone: false,
                },
            ),
        },
        ActionNode {
            id: 2,
            parent: Some(1),
            action: Action::new(
                Seat::North,
                ActionType::DealerDiscard,
                ActionData::Card { card: card("qs") },
            ),
        },
        ActionNode {
            id: 3,
            parent: Some(2),
            action: Action::new(
                Seat::East,
                ActionType::Lead,
                ActionData::Card { card: card("jc") },
            ),
        },
        ActionNode {
            id: 4,
            parent: Some(3),
            action: Action::new(
                Seat::South,
                ActionType::Follow,
                ActionData::Card { card: card("ac") },
            ),
        },
        ActionNode {
            id: 5,
            parent: Some(0),
            action: Action::new(Seat::South, ActionType::BidTop, ActionData::Pass),
        },
        ActionNode {
            id: 6,
            parent: Some(5),
            action: Action::new(Seat::West, ActionType::BidTop, ActionData::Pass),
        },
        ActionNode {
            id: 7,
            parent: Some(6),
            action: Action::new(
                Seat::North,
                ActionType::BidTop,
                ActionData::Call {
                    suit: Suit::Diamond,
                    alone: false,
                },
            ),
        },
        ActionNode {
            id: 8,
            parent: Some(7),
            action: Action::new(
                Seat::North,
                ActionType::DealerDiscard,
                ActionData::Card { card: card("qs") },
            ),
        },
        ActionNode {
            id: 9,
            parent: Some(8),
            action: Action::new(
                Seat::East,
                ActionType::Lead,
                ActionData::Card { card: card("jc") },
            ),
        },
        ActionNode {
            id: 10,
            parent: Some(9),
            action: Action::new(
                Seat::South,
                ActionType::Follow,
                ActionData::Card { card: card("ac") },
            ),
        },
        ActionNode {
            id: 11,
            parent: Some(6),
            action: Action::new(Seat::North, ActionType::BidTop, ActionData::Pass),
        },
        ActionNode {
            id: 12,
            parent: Some(11),
            action: Action::new(
                Seat::East,
                ActionType::BidOther,
                ActionData::Call {
                    suit: Suit::Club,
                    alone: false,
                },
            ),
        },
        ActionNode {
            id: 13,
            parent: Some(12),
            action: Action::new(
                Seat::East,
                ActionType::Lead,
                ActionData::Card { card: card("jc") },
            ),
        },
        ActionNode {
            id: 14,
            parent: None,
            action: Action::new(
                Seat::East,
                ActionType::BidTop,
                ActionData::Call {
                    suit: Suit::Diamond,
                    alone: false,
                },
            ),
        },
    ];
    RawLog { config, actions }
}

fn log_fixture() -> Log {
    raw_log_fixture().into()
}

#[test]
fn test_log_find_child() {
    let log = log_fixture();
    assert_eq!(
        Some(0),
        log.find_child(
            None,
            &Action::new(Seat::East, ActionType::BidTop, ActionData::Pass)
        )
    );
    assert_eq!(
        Some(14),
        log.find_child(
            None,
            &Action::new(
                Seat::East,
                ActionType::BidTop,
                ActionData::Call {
                    suit: Suit::Diamond,
                    alone: false
                }
            )
        )
    );
    assert_eq!(
        None,
        log.find_child(
            None,
            &Action::new(
                Seat::East,
                ActionType::BidTop,
                ActionData::Call {
                    suit: Suit::Diamond,
                    alone: true
                }
            )
        )
    );
    assert_eq!(
        Some(2),
        log.find_child(
            Some(1),
            &Action::new(
                Seat::North,
                ActionType::DealerDiscard,
                ActionData::Card { card: card("qs") },
            ),
        )
    );
    assert_eq!(
        Some(8),
        log.find_child(
            Some(7),
            &Action::new(
                Seat::North,
                ActionType::DealerDiscard,
                ActionData::Card { card: card("qs") },
            ),
        )
    );
    assert_eq!(
        None,
        log.find_child(
            Some(6),
            &Action::new(
                Seat::North,
                ActionType::DealerDiscard,
                ActionData::Card { card: card("qs") },
            ),
        )
    );
}

#[test]
fn test_log_backtrace() {
    let log = log_fixture();

    fn bt(log: &Log, id: Id) -> Vec<Id> {
        log.backtrace(id)
            .unwrap()
            .into_iter()
            .map(|x| x.0)
            .collect()
    }

    assert_eq!(bt(&log, 0), vec![0]);
    assert_eq!(bt(&log, 9), vec![0, 5, 6, 7, 8, 9]);
    assert_eq!(bt(&log, 12), vec![0, 5, 6, 11, 12]);
    assert_eq!(bt(&log, 14), vec![14]);
    assert!(log.backtrace(15).is_err());
}

#[test]
fn test_log_insert() {
    let mut log = log_fixture();
    let id = log.insert(
        None,
        Action::new(
            Seat::East,
            ActionType::BidTop,
            ActionData::Call {
                suit: Suit::Diamond,
                alone: false,
            },
        ),
    );
    assert_eq!(id, 14);
    let id = log.insert(
        None,
        Action::new(
            Seat::East,
            ActionType::BidTop,
            ActionData::Call {
                suit: Suit::Diamond,
                alone: true,
            },
        ),
    );
    assert_eq!(id, 15);
}

#[test]
fn test_log_serde() {
    let raw = raw_log_fixture();
    let ser = serde_json::to_string(&raw).unwrap();
    let de: RawLog = serde_json::from_str(&ser).unwrap();
    assert_eq!(raw, de);
}

#[test]
fn test_traverse() {
    let log = log_fixture();
    let nodes: Vec<_> = log.traverse().map(|n| n.id).collect();
    assert_eq!(nodes, (0..=14).collect::<Vec<_>>());
    let nodes: Vec<_> = log
        .traverse()
        .map(|n| (n.id, n.sibling, n.last_sibling, n.leaf))
        .collect();
    assert_eq!(
        nodes,
        vec![
            (0, true, false, false),
            (1, true, false, false),
            (2, false, false, false),
            (3, false, false, false),
            (4, false, false, true),
            (5, true, true, false),
            (6, false, false, false),
            (7, true, false, false),
            (8, false, false, false),
            (9, false, false, false),
            (10, false, false, true),
            (11, true, true, false),
            (12, false, false, false),
            (13, false, false, true),
            (14, true, true, true),
        ]
    );
}
