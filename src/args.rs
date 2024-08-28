//! Command line arguments

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Which game to play.
    #[arg(short, long)]
    pub game: Option<Game>,

    /// Which UI to use.
    #[arg(short, long)]
    pub ui: Option<Ui>,

    /// Log file to load.
    #[arg(short, long)]
    pub load: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum Game {
    /// The game of euchre.
    #[default]
    Euchre,
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum Ui {
    /// A very simple command line interface.
    Cli,
    /// A full-featured terminal UI.
    #[default]
    Tui,
}
