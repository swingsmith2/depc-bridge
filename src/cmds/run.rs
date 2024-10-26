use clap::Parser;

#[derive(Parser)]
pub struct Run {
    /// The address:port the web service will listen to
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    /// The endpoint (http://ip:port) for depc node
    #[arg(long, default_value = "http://127.0.0.1:18732")]
    pub depc_rpc_endpoint: String,
    /// Use cookie for RPC authentication
    #[arg(long, default_value_t = true)]
    pub depc_rpc_use_cookie: bool,
    /// The path string to file `.cookie`
    #[arg(long, default_value = "$HOME/.depinc/testnet3/.cookie")]
    pub depc_rpc_cookie_path: String,
    /// The username for RPC authentication
    #[arg(long, default_value = "")]
    pub depc_rpc_user: String,
    /// The password for RPC authentication
    #[arg(long, default_value = "")]
    pub depc_rpc_passwd: String,
    /// Use proxy for the connection of RPC
    #[arg(long, default_value_t = false)]
    pub depc_rpc_use_proxy: bool,
    #[arg(long)]
    pub depc_owner_address: String,
    /// The endpoint string should be used for establishing connection to solana node
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    pub sol_endpoint: String,
    /// The authority private key for manipulate spl-token from sonala network
    #[arg(long)]
    pub sol_authority_key: String,
    /// The mint address of the spl-token
    #[arg(long)]
    pub sol_mint_pubkey: String,
    /// The path string to local database
    #[arg(long, default_value = "$HOME/depc-bridge.sqlite3")]
    pub local_db: String,
    /// Monitor the chain for the owner address
    #[arg(long, default_value = "2NGWAccrksGM4TmefLN4qyW1kV7VpMngtBQ")]
    pub owner_address: String,
    /// The endpoint string will be used to establish connection for ethereum calls/transactions
    #[arg(
        long,
        default_value = "https://sepolia.infura.io/v3/daad1c45f9b6487288f56ff2bac9577a"
    )]
    pub eth_endpoint: String,
    /// The contract address represent the erc20 contract
    #[arg(long)]
    pub eth_contract_address: String,
    /// The private key to make signature
    #[arg(long)]
    pub eth_private_key: String,
}