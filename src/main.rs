mod chain;
mod db;
mod sync;

use std::sync::Arc;

use anyhow::Result;
use clap::{command, Parser};
use tokio::{sync::Mutex, time::Duration};

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
    #[arg(long, default_value = "$HOME/.depinc/testnet3/.cookie")]
    rpc_cookie_path: String,
    /// The username for RPC authentication
    #[arg(long)]
    rpc_user: String,
    /// The password for RPC authentication
    #[arg(long)]
    rpc_passwd: String,
    /// Use proxy for the connection of RPC
    #[arg(long, default_value_t = false)]
    rpc_use_proxy: bool,
    /// The path string to local database
    #[arg(long, default_value = "$HOME/depc-bridge.sqlite3")]
    local_db: String,
    /// Monitor the chain for the owner address
    #[arg(long, default_value = "2NGWAccrksGM4TmefLN4qyW1kV7VpMngtBQ")]
    owner_address: String,
}

async fn syncing_routine(
    conn: db::Conn,
    client: chain::Client,
    owner_address: String,
    exit_sig: Arc<Mutex<bool>>,
) -> Result<()> {
    loop {
        {
            let exit = exit_sig.lock().await;
            if *exit {
                break;
            }
        }
        sync::sync(&conn, &client, &owner_address, Arc::clone(&exit_sig)).await?;
        // check to exit
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = if args.rpc_use_cookie {
        chain::ClientBuilder::new()
            .set_auth_from_cookie(&args.rpc_cookie_path)
            .set_use_proxy(args.rpc_use_proxy)
            .set_endpoint(&args.rpc_endpoint)
            .build()
    } else {
        let auth_str = format!("{}:{}", &args.rpc_user, &args.rpc_passwd);
        chain::ClientBuilder::new()
            .set_auth(&auth_str)
            .set_use_proxy(args.rpc_use_proxy)
            .set_endpoint(&args.rpc_endpoint)
            .build()
    };

    let db_path = shellexpand::env(&args.local_db).unwrap();
    let conn = db::Conn::open_or_create(&db_path).unwrap();
    conn.init()?;

    let exit_sig = Arc::new(Mutex::new(false));

    tokio::spawn(syncing_routine(conn, client, args.owner_address, exit_sig)).await?
}
