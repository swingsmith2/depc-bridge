mod chain;

use clap::{command, Parser};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The endpoint (http://ip:port) for depc node
    #[arg(long)]
    rpc_endpoint: String,
    /// Use cookie for RPC authentication
    #[arg(long)]
    rpc_use_cookie: bool,
    /// The path string to file `.cookie`
    #[arg(long)]
    rpc_cookie_path: String,
    /// The username for RPC authentication
    #[arg(long)]
    rpc_user: String,
    /// The password for RPC authentication
    rpc_passwd: String,
    /// The path string to local database
    #[arg(long)]
    local_db: String,
}

#[tokio::main]
async fn main() {
    let _ = Args::parse();
}
