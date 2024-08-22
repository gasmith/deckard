use clap::Parser;

mod args;
mod euchre;
mod french;
use self::args::{Args, Ui};
use self::euchre::{cli_main, tui_main};

fn main() {
    let args = Args::parse();
    match args.ui.unwrap_or_default() {
        Ui::Console => cli_main(),
        Ui::Tui => tui_main(),
    }
}
