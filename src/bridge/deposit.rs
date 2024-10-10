use tokio::{
    sync::mpsc::Receiver,
    time::{sleep, Duration},
};

use super::bridge::{Bridge, Address};

pub struct Deposit {
    pub erc20_address: Address,
    pub amount: u64,
}

pub async fn consumer(mut rx: Receiver<Deposit>, bridge: Bridge) {
    loop {
        if let Some(deposit) = rx.recv().await {
            todo!("now process the deposit");
        } else {
            sleep(Duration::from_millis(3)).await;
        }
    }
}
