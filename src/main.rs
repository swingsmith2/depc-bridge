use clap::{command, Parser};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// address (ip:port) for depc node
    #[arg(long)]
    depc_node: String,

    #[arg(long)]
    local_db: String,
}

fn main() {
    let _ = Args::parse();
}
