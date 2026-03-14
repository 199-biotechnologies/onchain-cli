use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use reqwest::Client;

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockscoutTx {
    pub hash: String,
    pub blockNumber: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub timeStamp: String,
}

#[derive(Debug, Deserialize)]
struct BlockscoutResponse {
    pub result: Vec<BlockscoutTx>,
    pub status: String,
}

pub struct BlockscoutClient {
    client: Client,
    base_url: String,
}

impl BlockscoutClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://arbitrum.blockscout.com/api".to_string()),
        }
    }

    pub async fn get_transactions(&self, address: &str) -> Result<Vec<BlockscoutTx>> {
        let url = format!(
            "{}?module=account&action=txlist&address={}",
            self.base_url, address
        );

        let response: BlockscoutResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .context("Failed to parse Blockscout response")?;

        if response.status != "1" {
            anyhow::bail!("Blockscout API returned error status");
        }

        Ok(response.result)
    }
}
