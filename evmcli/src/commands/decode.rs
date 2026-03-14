use comfy_table::Table;
use serde::Serialize;
use std::collections::HashMap;

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct DecodeResult {
    pub selector: String,
    pub function_name: Option<String>,
    pub raw_data: String,
    pub data_length: usize,
}

impl Tableable for DecodeResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.add_row(vec!["Selector", &self.selector]);
        table.add_row(vec!["Function", self.function_name.as_deref().unwrap_or("unknown")]);
        table.add_row(vec!["Data Length", &format!("{} bytes", self.data_length)]);
        table
    }
}

fn known_selectors() -> HashMap<[u8; 4], &'static str> {
    let mut m = HashMap::new();
    // ERC20
    m.insert(hex_selector("a9059cbb"), "transfer(address,uint256)");
    m.insert(hex_selector("095ea7b3"), "approve(address,uint256)");
    m.insert(hex_selector("23b872dd"), "transferFrom(address,address,uint256)");
    m.insert(hex_selector("70a08231"), "balanceOf(address)");
    m.insert(hex_selector("dd62ed3e"), "allowance(address,address)");
    m.insert(hex_selector("313ce567"), "decimals()");
    m.insert(hex_selector("95d89b41"), "symbol()");
    m.insert(hex_selector("06fdde03"), "name()");
    m.insert(hex_selector("18160ddd"), "totalSupply()");
    // Uniswap V3
    m.insert(hex_selector("414bf389"), "exactInputSingle(...)");
    m.insert(hex_selector("db3e2198"), "exactOutputSingle(...)");
    m.insert(hex_selector("ac9650d8"), "multicall(bytes[])");
    m.insert(hex_selector("128acb08"), "swap(address,bool,int256,uint160,bytes)");
    // Aave V3
    m.insert(hex_selector("00a718a9"), "liquidationCall(address,address,address,uint256,bool)");
    m.insert(hex_selector("617ba037"), "supply(address,uint256,address,uint16)");
    m.insert(hex_selector("a415bcad"), "borrow(address,uint256,uint256,uint16,address)");
    m.insert(hex_selector("573ade81"), "repay(address,uint256,uint256,address)");
    m.insert(hex_selector("ab9c4b5d"), "flashLoan(address,address[],uint256[],uint256[],address,bytes,uint16)");
    // Balancer V2
    m.insert(hex_selector("5c38449e"), "flashLoan(address,address[],uint256[],bytes)");
    m.insert(hex_selector("52bbbe29"), "swap(...)");
    m.insert(hex_selector("945bcec9"), "batchSwap(...)");
    // Multicall3
    m.insert(hex_selector("82ad56cb"), "aggregate3(...)");
    m.insert(hex_selector("399542e9"), "tryAggregate(bool,...)");
    // Common
    m.insert(hex_selector("8da5cb5b"), "owner()");
    m.insert(hex_selector("f2fde38b"), "transferOwnership(address)");
    m.insert(hex_selector("715018a6"), "renounceOwnership()");
    m
}

fn hex_selector(hex: &str) -> [u8; 4] {
    let bytes = hex::decode(hex).unwrap();
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

pub async fn run(_ctx: &AppContext, data: &str) -> Result<DecodeResult, EvmError> {
    let data_clean = data.strip_prefix("0x").unwrap_or(data);
    let bytes = hex::decode(data_clean)
        .map_err(|_| EvmError::decode("Invalid hex data"))?;

    if bytes.len() < 4 {
        return Err(EvmError::decode("Calldata must be at least 4 bytes (function selector)"));
    }

    let selector: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    let selector_hex = format!("0x{}", hex::encode(selector));

    // Check baked-in selectors first
    let known = known_selectors();
    let function_name = if let Some(name) = known.get(&selector) {
        Some(name.to_string())
    } else {
        // Try samczsun's 4byte API
        lookup_4byte(&selector_hex).await
    };

    Ok(DecodeResult {
        selector: selector_hex,
        function_name,
        raw_data: format!("0x{}", data_clean),
        data_length: bytes.len(),
    })
}

async fn lookup_4byte(selector: &str) -> Option<String> {
    let url = format!("https://api.openchain.xyz/signature-database/v1/lookup?function={selector}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build().ok()?;
    let resp = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let results = json.get("result")?.get("function")?.get(selector)?;
    results.as_array()?.first()?.get("name")?.as_str().map(|s| s.to_string())
}
