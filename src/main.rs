use clap::{Parser, Subcommand};
use anyhow::Result;
use std::time::Instant;
use alloy::providers::Provider;
use alloy::rpc::types::eth::BlockTransactionsKind;

mod rpc;
mod commands;
mod blockscout;

#[derive(Parser)]
#[command(name = "evmtool")]
#[command(about = "High-performance EVM CLI tool for Arbitrum", long_about = None)]
struct Cli {
    #[arg(short, long)]
    rpc: Option<String>,

    #[arg(short, long, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get ETH balance of an address
    Balance {
        address: String,
    },
    /// Get transaction details
    Tx {
        hash: String,
    },
    /// Get block details
    Block {
        number_or_hash: Option<String>,
    },
    /// List transactions from Blockscout
    List {
        address: String,
    },
    /// Run performance benchmark
    Benchmark,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let start = Instant::now();

    let provider = rpc::ProviderFactory::create(cli.rpc).await?;

    match cli.command {
        Commands::Balance { address } => {
            let addr = address.parse()?;
            let balance = provider.get_balance(addr).await?;
            if cli.json {
                println!(r#"{{"address": "{}", "balance": "{}"}}"#, address, balance);
            } else {
                println!("Address: {}", address);
                println!("Balance: {} ETH", alloy::primitives::utils::format_ether(balance));
            }
        }
        Commands::Tx { hash } => {
            let tx_hash = hash.parse()?;
            let tx = provider.get_transaction_by_hash(tx_hash).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&tx)?);
            } else {
                match tx {
                    Some(t) => println!("Transaction: {:?}", t),
                    None => println!("Transaction not found: {}", hash),
                }
            }
        }
        Commands::Block { number_or_hash } => {
            let block = if let Some(val) = number_or_hash {
                if let Ok(num) = val.parse::<u64>() {
                    provider.get_block_by_number(num.into(), false).await?
                } else {
                    provider.get_block_by_hash(val.parse()?, BlockTransactionsKind::Hashes).await?
                }
            } else {
                provider.get_block_by_number(alloy::rpc::types::BlockNumberOrTag::Latest, false).await?
            };

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&block)?);
            } else {
                match block {
                    Some(b) => println!("Block: {:?}", b.header),
                    None => println!("Block not found"),
                }
            }
        }
        Commands::List { address } => {
            let client = blockscout::BlockscoutClient::new(None);
            let txs = client.get_transactions(&address).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&txs)?);
            } else {
                println!("Recent Transactions for {}:", address);
                for tx in txs.iter().take(5) {
                    println!("  {} | Value: {} | Block: {}", tx.hash, tx.value, tx.blockNumber);
                }
            }
        }
        Commands::Benchmark => {
            println!("Running benchmark (10 iterations)...");
            let start_bench = Instant::now();
            for _ in 0..10 {
                let _ = provider.get_block_number().await?;
            }
            let duration = start_bench.elapsed();
            if cli.json {
                println!(r#"{{"total_ms": {}, "avg_ms": {}}}"#, duration.as_millis(), duration.as_millis() / 10);
            } else {
                println!("Total time: {:?}", duration);
                println!("Average per request: {:?}", duration / 10);
            }
        }
    }

    if !cli.json {
        eprintln!("\nTotal execution time: {:?}", start.elapsed());
    }

    Ok(())
}
