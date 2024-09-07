#![allow(clippy::module_name_repetitions, clippy::struct_field_names)]

use clap::Parser;

mod args;
mod deck;
mod euchre;
mod french;
use self::args::{Args, Game, Ui};

fn main() {
    let args = Args::parse();
    match (args.game.unwrap_or_default(), args.ui.unwrap_or_default()) {
        (Game::Euchre, Ui::Cli) => euchre::cli_main(),
        (Game::Euchre, Ui::Tui) => euchre::tui_main(args.load.as_deref()),
    }
}
