use std::{collections::HashMap, sync::{Arc, Mutex}};

use axum::{extract::{Path, State}, routing::get, Json, Router};
use chrono::DateTime;
use log::{error, info, warn};
use num_format::{Locale, ToFormattedString};
use serde_json::Value;
use tokio::signal;

use crate::db;

struct ServerData {
    conn: db::Conn,
    exit: Arc<Mutex<bool>>,
}

trait FormatMoney {
    fn format_money(&self) -> String;
}

impl FormatMoney for u64 {
    fn format_money(&self) -> String {
        const COIN: u64 = 100000000;
        (self / COIN).to_formatted_string(&Locale::en)
    }
}

#[axum::debug_handler]
async fn get_root() -> &'static str {
    "hello world"
}

#[derive(serde::Serialize)]
struct RespExchangeBalanceByDate {
    balance: u64,
    balance_human: String,
    addresses: HashMap<String, String>,
}

#[derive(serde::Serialize)]
struct RespExchangeAddresses {
    total: u64,
    saved: u64,
}

#[axum::debug_handler]
async fn get_exchange_addresses(
    Path(txid): Path<String>,
    State(state): State<Arc<ServerData>>,
) -> Json<Value> {
    let mut final_addresses = vec![];
    let addresses = state.conn.query_inputs(&txid).unwrap();
    final_addresses.extend(addresses.clone());
    info!(
        "queried total {} address(es) from txid {}",
        final_addresses.len(),
        txid
    );
    let mut final_txids = Vec::new();
    for address in addresses.iter() {
        tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
        {
            let exit = state.exit.lock().unwrap();
            if *exit {
                break;
            }
        }
        info!("querying txids which are related to address {}", address);
        let txids = state
            .conn
            .query_txids_those_inputs_contain_address(address)
            .unwrap();
        info!(
            "queried total {} txid(s) which are related to address {}",
            txids.len(),
            address
        );
        final_txids.extend(txids);
    }
    final_txids.sort();
    final_txids.dedup();
    info!(
        "analyzing total {} txids to get exchange addresses",
        final_txids.len()
    );
    let mut total_saved = 0u64;
    for txid in final_txids.iter() {
        tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
        {
            let exit = state.exit.lock().unwrap();
            if *exit {
                break;
            }
        }
        let sub_addresses = state.conn.query_inputs(txid).unwrap();
        info!(
            "appending total {} address(es) into database",
            sub_addresses.len()
        );
        for address in sub_addresses {
            let res = state
                .conn
                .add_analyzed_exchange_address_from_tx(&address, txid);
            if res.is_err() {
                error!(
                    "append related address {} from tx {} is failed, reason: {:?}",
                    address,
                    txid,
                    res.err()
                );
            } else {
                info!("saved {} from tx {} into database", address, txid);
                total_saved += 1;
            }
        }
    }
    info!("result is ready.");

    Json(
        serde_json::to_value(RespExchangeAddresses {
            saved: total_saved,
            total: state.conn.query_num_exchange_addresses().unwrap(),
        })
        .unwrap(),
    )
}

#[axum::debug_handler]
async fn generate_exchange_balances(
    Path(days): Path<String>,
    State(state): State<Arc<ServerData>>,
) -> Json<Value> {
    let days = days.parse().unwrap_or(7);
    // query balances with different period
    const HEIGHTS_DAY: u32 = 60 / 3 * 24;
    const MIN_HEIGHT: u32 = 860130u32;
    let heights_period: u32 = HEIGHTS_DAY * days;
    let mut resp = HashMap::new();
    let chain_height = state.conn.query_best_height().unwrap_or_default();
    let mut curr_height = MIN_HEIGHT;
    'outer: loop {
        let block_timestamp = state.conn.query_block_time_by_height(curr_height);
        let now = DateTime::from_timestamp(block_timestamp as i64, 0).unwrap();
        info!("checking balance for date {}...", now.to_rfc3339());
        let mut balance_by_date = RespExchangeBalanceByDate {
            balance: 0,
            balance_human: 0u64.format_money(),
            addresses: HashMap::new(),
        };
        let final_addresses = state.conn.query_analyzed_exchange_addresses().unwrap();
        info!("total {} exchange address(es) found", final_addresses.len());
        for address in final_addresses.iter() {
            tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
            {
                let exit = state.exit.lock().unwrap();
                if *exit {
                    break 'outer;
                }
            }
            let curr_balance = state
                .conn
                .query_balance(address, curr_height)
                .unwrap_or_default();
            if curr_balance > 0 {
                balance_by_date.balance += curr_balance;
                balance_by_date
                    .addresses
                    .insert(address.clone(), curr_balance.format_money());
            }
        }
        balance_by_date.balance_human = balance_by_date.balance.format_money();
        info!("checked, balance = {}", balance_by_date.balance_human);

        // save to resp
        resp.insert(now.to_rfc3339(), balance_by_date);
        // next
        curr_height += heights_period;
        if curr_height > chain_height {
            break;
        }
    }
    info!("done.");

    Json(serde_json::to_value(resp).unwrap())
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

pub async fn run_service(bind: &str, conn: db::Conn, exit_sig: Arc<Mutex<bool>>) {
    info!("listening on {}", bind);
    let app = Router::new()
        .route("/", get(get_root))
        .route("/exchange/analyze/:txid", get(get_exchange_addresses))
        .route("/exchange/balances/:days", get(generate_exchange_balances))
        .with_state(Arc::new(ServerData {
            conn,
            exit: Arc::clone(&exit_sig),
        }));
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();

    info!("web server is running...");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(exit_sig))
        .await
        .unwrap();

    info!("web server exits.");
}
