use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::DateTime;
use log::{error, info, warn};
use num_format::{Locale, ToFormattedString};
use serde::Serialize;
use serde_json::Value;
use std::str::FromStr;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::signal;

use serde_json::json;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{
    db,
    solana::{AnalyzedInstruction, InstructionDetail, SolanaClient},
};

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

#[derive(Serialize)]
struct RespExchangeBalanceByDate {
    balance: u64,
    balance_human: String,
    addresses: HashMap<String, String>,
}

#[derive(Serialize)]
struct RespExchangeAddresses {
    total: u64,
    saved: u64,
}

#[derive(Serialize)]
struct BalanceResponse {
    address: String,
    balance: u64,
}

#[derive(Serialize)]
struct UploadTransactionResponse {
    result: String,
}

#[derive(Serialize)]
struct TransactionDetail {
    signature: String,
    source: String,
    destination: String,
    amount: u64,
    fee: u64,
    timestamp: i64,
    r#type: String,
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
    let res = params.get("address");
    if res.is_none() {
        // no 'address' can be found from parameter list, return errors
        return Json(make_error_json(
            0,
            "no 'address' can be found from parameter list".to_owned(),
        ));
    }
    let mut balances = vec![];

    let iter = res.unwrap().split(",");
    for address in iter {
        let res = Pubkey::from_str(address);
        if res.is_err() {
            return Json(make_error_json(
                0,
                format!("cannot parse address from string '{}'", address),
            ));
        }
        let pubkey = res.unwrap();
        if let Ok(balance) = state.solana_client.get_balance(&pubkey) {
            let resp = BalanceResponse {
                address: address.to_owned(),
                balance,
            };
            let value = serde_json::to_value(resp).unwrap();
            balances.push(value);
        } else {
            let value =
                make_error_json(0, format!("cannot get balance for address: '{}'", address));
            balances.push(value);
        }
    }
    Json(json!(balances))
}

#[axum::debug_handler]
async fn get_solana_history(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerData>>,
) -> Json<Value> {
    let res = params.get("address");
    if res.is_none() {
        // no 'address' can be found from parameter list, return errors
        return Json(make_error_json(
            0,
            "no 'address' can be found from parameter list".to_owned(),
        ));
    }
    let mut parsed_transactions = vec![];
    let iter = res.unwrap().split(",");
    for address in iter {
        let res = Pubkey::from_str(address);
        if res.is_err() {
            // invalid address, need to return err
            return Json(make_error_json(
                0,
                format!("cannot parse address from string '{}'", address),
            ));
        }
        let pubkey = res.unwrap();
        let res = state
            .solana_client
            .get_transactions_related_to_address(&pubkey);
        if let Err(e) = res {
            return Json(make_error_json(
                0,
                format!(
                    "cannot parse or get transactions related to address {}, reason: {}",
                    address, e
                ),
            ));
        }
        let analyzed_transactions = res.unwrap();
        for analyzed_transaction in analyzed_transactions.iter() {
            for ix in analyzed_transaction.instructions.iter() {
                let transaction_detail = match ix {
                    AnalyzedInstruction::SplToken(ix_detail) => make_transaction_detail(
                        ix_detail,
                        &analyzed_transaction.signature,
                        analyzed_transaction.fee,
                        analyzed_transaction.timestamp,
                        "token".to_owned(),
                    ),
                    AnalyzedInstruction::Solana(ix_detail) => make_transaction_detail(
                        ix_detail,
                        &analyzed_transaction.signature,
                        analyzed_transaction.fee,
                        analyzed_transaction.timestamp,
                        "sol".to_owned(),
                    ),
                };
                parsed_transactions.push(transaction_detail);
            }
        }
    }
    Json(json!(parsed_transactions))
}

#[axum::debug_handler]
async fn post_solana_transaction(
    State(state): State<Arc<ServerData>>,
    Json(base64_data): Json<String>,
) -> Json<Value> {
    let res = base64::decode(&base64_data);
    if res.is_err() {
        return Json(make_error_json(0, "cannot decode base64 data".to_owned()));
    }
    let bytes = res.unwrap();
    let res = bincode::deserialize(&bytes);
    if res.is_err() {
        // cannot deserialize the binary code into transaction
        return Json(make_error_json(0, "invalid transaction data".to_owned()));
    }
    let transaction = res.unwrap();
    if let Ok(signature) = state.solana_client.upload_transaction(&transaction) {
        Json(json!(UploadTransactionResponse {
            result: signature.to_string(),
        }))
    } else {
        Json(make_error_json(
            0,
            "failed to upload transaction".to_owned(),
        ))
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

fn make_transaction_detail(
    ix_detail: &InstructionDetail,
    signature: &Signature,
    fee: u64,
    timestamp: i64,
    r#type: String,
) -> TransactionDetail {
    TransactionDetail {
        signature: signature.to_string(),
        source: ix_detail.source.to_string(),
        destination: ix_detail.destination.to_string(),
        amount: ix_detail.amount,
        fee,
        timestamp,
        r#type,
    }
}

#[derive(Serialize)]
struct ErrorDetail {
    code: u32,
    message: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

fn make_error_json(code: u32, message: String) -> Value {
    serde_json::to_value(ErrorResponse {
        error: ErrorDetail { code, message },
    })
    .unwrap()
}
