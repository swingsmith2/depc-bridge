use clap::{Parser, Subcommand};

use super::cmds::{Deploy, Run};

#[derive(Subcommand)]
pub enum Commands {
    Run(Run),
    Deploy(Deploy),
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}
