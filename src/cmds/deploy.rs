use clap::Parser;

#[derive(Parser)]
pub struct Deploy {
    /// The endpoint string for establishing connection to eth node.
    #[arg(short, long)]
    pub eth_endpoint: String,
}
