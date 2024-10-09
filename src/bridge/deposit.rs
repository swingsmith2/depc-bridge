use tokio::{
    sync::mpsc::Receiver,
    time::{sleep, Duration},
};

pub struct Deposit {
    pub erc20_address: String,
    pub amount: u64,
}

pub async fn consumer(mut rx: Receiver<Deposit>) {
    loop {
        if let Some(deposit) = rx.recv().await {
            todo!("now process the deposit");
        } else {
            sleep(Duration::from_millis(3)).await;
        }
    }
}
