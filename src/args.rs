//! Command line arguments

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub ui: Option<Ui>,
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum Ui {
    Console,
    #[default]
    Tui,
}
