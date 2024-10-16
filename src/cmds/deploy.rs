use clap::Parser;

#[derive(Parser)]
pub struct Deploy {
    /// The endpoint string for establishing connection to eth node.
    #[arg(short, long)]
    pub eth_endpoint: String,
    /// The private key string will be used to upload contract.
    #[arg(short, long)]
    pub eth_private_key: String,
}
