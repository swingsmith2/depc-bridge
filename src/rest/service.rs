use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::DateTime;
use log::{error, info, warn};
use num_format::{Locale, ToFormattedString};
use serde_json::Value;
use std::str::FromStr;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::signal;

use crate::db;
use crate::solana::{parse_signatures_for_target, SolanaClient, TransactionDetail};

use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{signature::Signature, transaction::Transaction};
use solana_transaction_status::{
    EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,
};

const SUCESS_CODE: i32 = 200;
const ERROR_CODE: i32 = 3000;
#[derive(Clone)]
struct ServerData {
    conn: db::Conn,
    solana_client: SolanaClient,
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

#[derive(serde::Serialize)]
struct BalanceResponse {
    code: Option<i32>,
    msg: Option<String>,
    address: Option<String>,
    balance: Option<u64>,
}

#[derive(serde::Serialize)]
struct TransactionResponse {
    code: Option<i32>,
    msg: Option<String>,
    result: Option<String>,
}

#[derive(serde::Serialize)]
struct HistoryResponse {
    code: Option<i32>,
    msg: Option<String>,
    result: Option<Vec<TransactionDetail>>,
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

#[axum::debug_handler]
async fn get_solana_balance(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerData>>,
) -> Json<Value> {
    let empty_string = "".to_string();
    let addresses = params.get("address").unwrap_or(&empty_string).split(",");
    let mut balances = vec![];

    for address in addresses {
        let mut flag = true;
        let balance = match state
            .solana_client
            .rpc_client
            .get_balance(&address.parse().unwrap())
        {
            Ok(bal) => bal,
            Err(e) => {
                error!("Failed to get balance for {}: {:?}", address, e);
                flag = false;
                0
            }
        };
        if flag {
            balances.push(BalanceResponse {
                code: None,
                msg: None,
                address: Some(address.to_string()),
                balance: Some(balance),
            });
        } else {
            balances.push(BalanceResponse {
                code: Some(ERROR_CODE),
                msg: Some("fail".to_string()),
                address: Some(address.to_string()),
                balance: Some(0),
            });
        }
    }

    Json(json!(balances))
}
#[axum::debug_handler]
pub async fn get_solana_history(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerData>>,
) -> Json<Value> {
    let mut parsed_transactions = vec![];
    let emptyStr = "".to_string();
    let addresses = params.get("address").unwrap_or(&emptyStr).split(",");

    for address in addresses {
        let pubkey = Pubkey::from_str(&address).unwrap();
        let signatures_res = state
            .solana_client
            .rpc_client
            .get_signatures_for_address(&pubkey)
            .unwrap();

        let transactions = state
            .solana_client
            .parse_signatures_for_target(signatures_res);
        match transactions {
            Ok(tx_details) => {
                parsed_transactions.push(HistoryResponse{
                    code: None,
                    msg: None,
                    result: Some(tx_details)
                });
            }
            Err(err) => {
                parsed_transactions.push(HistoryResponse{
                    code: Some(ERROR_CODE),
                    msg: Some("fail".to_string()),
                    result: None
                });
            }
        }
    }
    Json(json!(parsed_transactions))
}
#[axum::debug_handler]
async fn post_solana_transaction(
    State(state): State<Arc<ServerData>>,
    Json(tx_data): Json<String>,
) -> Json<Value> {
    let tx_bytes = base64::decode(&tx_data).unwrap();
    let tx: Transaction = bincode::deserialize(&tx_bytes).unwrap();

    match state.solana_client.rpc_client.send_transaction(&tx) {
        Ok(signature) => Json(json!(TransactionResponse {
            code: None,
            msg: None,
            result: Some(signature.to_string()),
        })),
        Err(e) => Json(json!(TransactionResponse {
            code: Some(ERROR_CODE),
            msg: Some("fail".to_string()),
            result: Some("".to_string()),
        })),
    }
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

pub async fn run_service(
    bind: &str,
    conn: db::Conn,
    solana_client: SolanaClient,
    exit_sig: Arc<Mutex<bool>>,
) {
    info!("listening on {}", bind);
    let app = Router::new()
        .route("/", get(get_root))
        .route("/exchange/analyze/:txid", get(get_exchange_addresses))
        .route("/exchange/balances/:days", get(generate_exchange_balances))
        .route("/solana/balance", get(get_solana_balance))
        .route("/solana/history", get(get_solana_history))
        .route("/solana/post_tx", post(post_solana_transaction))
        .with_state(Arc::new(ServerData {
            conn,
            solana_client,
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
