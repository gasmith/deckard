pub mod french;
pub mod game;
use french::{Card, Deck};
use game::war::{DrawError, War};

fn main() {
    let mut rng = rand::thread_rng();
    let mut deck = Deck::standard().with_cards([Card::Joker, Card::Joker]);
    deck.shuffle(&mut rng);
    let mut war = War::new(deck, "a", "b");
    for i in 1.. {
        print!("round {i}: ");
        if let Err(DrawError::OutOfCards(name)) = war.play_round(&mut rng) {
            println!("player {name} is out of cards");
            break;
        }
    }
}
