mod depc;
mod solana;

mod bridge;

mod db;
mod rpc;

mod args;
mod cmds;

mod rest;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use rest::run_service;

use args::{Args, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    debug!("debug mode");

    let args = Args::parse();

    match args.command {
        Commands::Run(args) => {
            let client = if args.depc_rpc_use_cookie {
                let cookie_path = shellexpand::env(&args.depc_rpc_cookie_path).unwrap();
                info!(
                    "prepare client with cookie file {} to {}",
                    cookie_path, args.depc_rpc_endpoint
                );
                depc::ClientBuilder::new()
                    .set_auth_from_cookie(&cookie_path)
                    .set_use_proxy(args.depc_rpc_use_proxy)
                    .set_endpoint(&args.depc_rpc_endpoint)
                    .build()
            } else {
                info!(
                    "prepare client with user/passwd to {}",
                    args.depc_rpc_endpoint
                );
                let auth_str = format!("{}:{}", &args.depc_rpc_user, &args.depc_rpc_passwd);
                depc::ClientBuilder::new()
                    .set_auth(&auth_str)
                    .set_use_proxy(args.depc_rpc_use_proxy)
                    .set_endpoint(&args.depc_rpc_endpoint)
                    .build()
            };

            let db_path = shellexpand::env(&args.local_db).unwrap();
            let conn = db::Conn::open_or_create(&db_path).unwrap();
            conn.init()?;
            info!("connected to local database, path {}", db_path);

            let exit_sig = Arc::new(Mutex::new(false));

            // TODO create bridge here

            // running webservice
            run_service(&args.bind, conn, exit_sig).await;

            info!("exit.");
            Ok(())
        }
        Commands::Deploy(deploy) => {
            todo!("complete this command")
        }
    }
}