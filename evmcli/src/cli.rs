use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "evmcli", version, about = "Fast EVM CLI toolkit")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output as JSON (auto-detected when piped)
    #[arg(long, global = true)]
    pub json: bool,

    /// Network name or chain ID (default: arbitrum)
    #[arg(long, global = true, default_value = "arbitrum", env = "EVMCLI_NETWORK")]
    pub network: String,

    /// Custom RPC URL (overrides auto-detect)
    #[arg(long, global = true, env = "EVMCLI_RPC_URL")]
    pub rpc_url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get native token or ERC20 balance
    Balance {
        /// Address to check
        address: String,
        /// ERC20 token contract address (omit for native balance)
        #[arg(long)]
        token: Option<String>,
    },

    /// Get transaction details by hash
    Tx {
        /// Transaction hash
        hash: String,
    },

    /// Get transaction receipt
    Receipt {
        /// Transaction hash
        hash: String,
    },

    /// Get block details
    Block {
        /// Block number, hash, or "latest"
        id: String,
    },

    /// Get current gas prices
    Gas,

    /// Read a smart contract (eth_call)
    Call {
        /// Contract address
        address: String,
        /// Function signature, e.g. "owner()(address)"
        sig: String,
        /// Function arguments
        args: Vec<String>,
    },

    /// List transactions from Blockscout
    Txs {
        /// Address to list transactions for
        address: String,
    },

    /// Decode calldata
    Decode {
        /// Calldata hex string (0x-prefixed)
        data: String,
    },

    /// Fetch and cache contract ABI
    Abi {
        /// Contract address
        address: String,
    },

    /// Run performance benchmark
    Bench {
        /// Number of iterations
        #[arg(long, default_value = "20")]
        iterations: u32,
        /// Warmup iterations
        #[arg(long, default_value = "3")]
        warmup: u32,
        /// Address to benchmark with
        #[arg(long, default_value = "0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A")]
        address: String,
    },
}
