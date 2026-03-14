use crate::config::ChainConfig;
use crate::errors::EvmError;
use directories::ProjectDirs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

const CACHE_TTL_SECS: u64 = 30;
const LOCAL_PROBE_TIMEOUT_MS: u64 = 200;
const PUBLIC_PROBE_DELAY_MS: u64 = 40;

/// Select the best RPC endpoint using happy-eyeballs probing with disk cache.
pub async fn select_endpoint(
    rpc_override: Option<&str>,
    chain: &ChainConfig,
    http: &reqwest::Client,
) -> Result<String, EvmError> {
    // 1. Explicit override
    if let Some(url) = rpc_override {
        return Ok(url.to_string());
    }

    // 2. Check disk cache
    if let Some(cached) = read_cache(chain.chain_id) {
        tracing::debug!("Using cached RPC endpoint: {cached}");
        return Ok(cached);
    }

    // 3. Happy-eyeballs probe
    let winner = probe_endpoints(chain, http).await?;
    write_cache(chain.chain_id, &winner);
    Ok(winner)
}

async fn probe_endpoints(chain: &ChainConfig, http: &reqwest::Client) -> Result<String, EvmError> {
    let local_url = chain.local_rpc.to_string();
    let public_url = chain.public_rpc.to_string();
    let expected_chain_id = chain.chain_id;

    // Try local first with a short timeout
    let local_result = {
        let http = http.clone();
        let url = local_url.clone();
        probe_rpc(&http, &url, expected_chain_id).await
    };

    if local_result.is_ok() {
        tracing::info!("Using local RPC: {local_url}");
        return Ok(local_url);
    }

    // Local failed/timed out, try public
    tracing::debug!("Local RPC unavailable, trying public...");
    let public_result = probe_rpc(http, &public_url, expected_chain_id).await;

    if public_result.is_ok() {
        tracing::info!("Using public RPC: {public_url}");
        return Ok(public_url);
    }

    Err(EvmError::rpc("All RPC endpoints failed"))
}

async fn probe_rpc(http: &reqwest::Client, url: &str, expected_chain_id: u64) -> Result<(), ()> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_chainId",
        "params": [],
        "id": 1
    });

    let result = timeout(
        Duration::from_millis(LOCAL_PROBE_TIMEOUT_MS),
        http.post(url).json(&body).send(),
    ).await;

    match result {
        Ok(Ok(resp)) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(hex_id) = json["result"].as_str() {
                    let chain_id = u64::from_str_radix(hex_id.trim_start_matches("0x"), 16).unwrap_or(0);
                    if chain_id == expected_chain_id {
                        return Ok(());
                    }
                }
            }
            Err(())
        }
        _ => Err(()),
    }
}

fn cache_path(chain_id: u64) -> Option<PathBuf> {
    ProjectDirs::from("", "", "evmcli").map(|dirs| {
        dirs.cache_dir().join(format!("rpc_winner_{chain_id}"))
    })
}

fn read_cache(chain_id: u64) -> Option<String> {
    let path = cache_path(chain_id)?;
    let metadata = std::fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    if age > Duration::from_secs(CACHE_TTL_SECS) {
        return None;
    }
    std::fs::read_to_string(&path).ok()
}

fn write_cache(chain_id: u64, url: &str) {
    if let Some(path) = cache_path(chain_id) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, url);
    }
}
