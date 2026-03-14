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

    /// Get event logs (Transfer, Swap, etc.)
    Logs {
        /// Contract address to filter logs from
        #[arg(long)]
        address: Option<String>,
        /// Event topic0 hash (e.g. Transfer topic)
        #[arg(long)]
        topic0: Option<String>,
        /// Filter by a specific address in topic1 or topic2
        #[arg(long)]
        participant: Option<String>,
        /// Start block (default: latest - 1000)
        #[arg(long)]
        from_block: Option<u64>,
        /// End block (default: latest)
        #[arg(long)]
        to_block: Option<u64>,
        /// Shorthand: --event transfer|approval|swap
        #[arg(long)]
        event: Option<String>,
    },

    /// Get token transfer history from Blockscout
    Transfers {
        /// Address to get transfers for
        address: String,
        /// Filter by token type: erc20, erc721, erc1155
        #[arg(long, default_value = "erc20")]
        token_type: String,
    },

    /// Read raw storage slot
    Storage {
        /// Contract address
        address: String,
        /// Storage slot (hex, e.g. 0x0)
        slot: String,
        /// Block number (default: latest)
        #[arg(long)]
        block: Option<u64>,
    },

    /// Get transaction count (nonce) for an address
    Nonce {
        /// Address
        address: String,
    },

    /// Check if address is EOA or contract
    Code {
        /// Address to check
        address: String,
    },

    /// Trace internal calls of a transaction (requires archive node)
    Trace {
        /// Transaction hash
        hash: String,
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
