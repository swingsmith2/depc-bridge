mod chain;
mod db;
mod sync;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::{routing::get, Router};
use clap::{command, Parser};
use log::{debug, info, warn};
use tokio::{signal, time::Duration};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The address:port the web service will listen to
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: String,
    /// The endpoint (http://ip:port) for depc node
    #[arg(long, default_value = "http://127.0.0.1:18732")]
    rpc_endpoint: String,
    /// Use cookie for RPC authentication
    #[arg(long, default_value_t = true)]
    rpc_use_cookie: bool,
    /// The path string to file `.cookie`
    #[arg(long, default_value = "$HOME/.depinc/testnet3/.cookie")]
    rpc_cookie_path: String,
    /// The username for RPC authentication
    #[arg(long, default_value = "")]
    rpc_user: String,
    /// The password for RPC authentication
    #[arg(long, default_value = "")]
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
            let exit = exit_sig.lock().unwrap();
            if *exit {
                info!("syncing loop exits.");
                break;
            }
        }
        sync::sync(&conn, &client, &owner_address, Arc::clone(&exit_sig)).await?;
        // check to exit
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn get_root() -> &'static str {
    "hello world"
}

async fn shutdown_signal(exit: Arc<Mutex<bool>>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        warn!("ctrl-c is received, sending exit signal");

        {
            let mut e = exit.lock().unwrap();
            *e = true;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    debug!("debug mode");

    let args = Args::parse();

    let client = if args.rpc_use_cookie {
        let cookie_path = shellexpand::env(&args.rpc_cookie_path).unwrap();
        info!(
            "prepare client with cookie file {} to {}",
            cookie_path, args.rpc_endpoint
        );
        chain::ClientBuilder::new()
            .set_auth_from_cookie(&cookie_path)
            .set_use_proxy(args.rpc_use_proxy)
            .set_endpoint(&args.rpc_endpoint)
            .build()
    } else {
        info!("prepare client with user/passwd to {}", args.rpc_endpoint);
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
    info!("connected to local database, path {}", db_path);

    let exit_sig = Arc::new(Mutex::new(false));

    let syncing_handler = tokio::spawn(syncing_routine(
        conn,
        client,
        args.owner_address,
        Arc::clone(&exit_sig),
    ));

    info!("listening on {}", args.bind);
    let app = Router::new().route("/", get(get_root));
    let listener = tokio::net::TcpListener::bind(args.bind).await.unwrap();

    info!("web server is running...");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(exit_sig))
        .await
        .unwrap();

    info!("web server exits.");
    syncing_handler.await.unwrap().unwrap();

    info!("exit.");
    Ok(())
}
