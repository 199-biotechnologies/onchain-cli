use alloy::providers::{Provider, ProviderBuilder};
use crate::errors::EvmError;
use std::time::Duration;

pub type ReadProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        alloy::providers::Identity,
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::GasFiller,
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::BlobGasFiller,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::NonceFiller,
                    alloy::providers::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy::providers::RootProvider,
>;

/// Build a shared reqwest client with connection pooling
pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client")
}

/// Build a read-only Alloy provider (no wallet)
pub async fn build_read_provider(rpc_url: &str) -> Result<ReadProvider, EvmError> {
    let provider = ProviderBuilder::new()
        .connect(rpc_url)
        .await
        .map_err(|e| EvmError::rpc(format!("Failed to connect to {rpc_url}: {e}")))?;
    Ok(provider)
}
