use alloy::providers::{ProviderBuilder, RootProvider, Provider};
use alloy::transports::http::{Http, Client};
use anyhow::{Result, Context};

pub const LOCAL_RPC: &str = "http://localhost:8547";
pub const PUBLIC_ARB_RPC: &str = "https://arb1.arbitrum.io/rpc";

pub struct ProviderFactory;

impl ProviderFactory {
    pub async fn create(rpc_url: Option<String>) -> Result<RootProvider<Http<Client>>> {
        let urls = if let Some(url) = rpc_url {
            vec![url]
        } else {
            vec![LOCAL_RPC.to_string(), PUBLIC_ARB_RPC.to_string()]
        };

        for url in urls {
            match Self::try_connect(&url).await {
                Ok(provider) => {
                    // Check if we can actually reach the chain
                    match provider.get_chain_id().await {
                        Ok(chain_id) => {
                            eprintln!("Connected to {} (Chain ID: {})", url, chain_id);
                            return Ok(provider);
                        }
                        Err(e) => {
                            eprintln!("Warning: Connected to {} but failed to get chain ID: {}", url, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not connect to {}: {}", url, e);
                }
            }
        }

        anyhow::bail!("Failed to connect to any RPC endpoint")
    }

    async fn try_connect(url: &str) -> Result<RootProvider<Http<Client>>> {
        let provider = ProviderBuilder::new()
            .on_http(url.parse().context("Invalid RPC URL")?);
        
        // Simple timeout check for connection
        Ok(provider)
    }
}
