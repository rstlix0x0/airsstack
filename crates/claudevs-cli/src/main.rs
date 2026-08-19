//! `claudevs` — Claude Code plugin lifecycle CLI (thin binary over the `claudevs` crate).

mod cli;

use clap::Parser as _;

fn main() {
    std::process::exit(cli::run(cli::Cli::parse()));
}
