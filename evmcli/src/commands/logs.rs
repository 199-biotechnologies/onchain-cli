use alloy::primitives::{Address, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use comfy_table::Table;
use serde::Serialize;

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

// Well-known event signatures
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const APPROVAL_TOPIC: &str = "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
const SWAP_V3_TOPIC: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
const SWAP_V2_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";

#[derive(Debug, Serialize)]
pub struct LogsResult {
    pub log_count: usize,
    pub from_block: u64,
    pub to_block: u64,
    pub logs: Vec<LogEntry>,
    pub rpc_endpoint: String,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub address: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub topic0: String,
    pub event_name: Option<String>,
    pub topics: Vec<String>,
    pub data_hex: String,
    pub log_index: u32,
}

impl Tableable for LogsResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Block", "TX", "Event", "Contract", "Data"]);
        for log in &self.logs {
            let tx_short = if log.tx_hash.len() > 14 {
                format!("{}...{}", &log.tx_hash[..8], &log.tx_hash[log.tx_hash.len()-4..])
            } else { log.tx_hash.clone() };
            let addr_short = if log.address.len() > 14 {
                format!("{}...{}", &log.address[..8], &log.address[log.address.len()-4..])
            } else { log.address.clone() };
            let data_short = if log.data_hex.len() > 20 {
                format!("{}... ({} bytes)", &log.data_hex[..18], (log.data_hex.len()-2)/2)
            } else { log.data_hex.clone() };
            table.add_row(vec![
                &log.block_number.to_string(),
                &tx_short,
                log.event_name.as_deref().unwrap_or(&log.topic0[..10]),
                &addr_short,
                &data_short,
            ]);
        }
        table.add_row(vec![&format!("{} logs", self.log_count), "", "", "", ""]);
        table
    }
}

fn resolve_event_topic(event: &str) -> Result<String, EvmError> {
    match event.to_lowercase().as_str() {
        "transfer" => Ok(TRANSFER_TOPIC.to_string()),
        "approval" | "approve" => Ok(APPROVAL_TOPIC.to_string()),
        "swap" | "swapv3" | "swap_v3" => Ok(SWAP_V3_TOPIC.to_string()),
        "swapv2" | "swap_v2" => Ok(SWAP_V2_TOPIC.to_string()),
        _ => Err(EvmError::validation(format!(
            "Unknown event shorthand: {event}. Use: transfer, approval, swap, swapv2. Or pass --topic0 directly."
        ))),
    }
}

fn topic0_to_name(topic: &str) -> Option<&'static str> {
    match topic {
        t if t == TRANSFER_TOPIC => Some("Transfer"),
        t if t == APPROVAL_TOPIC => Some("Approval"),
        t if t == SWAP_V3_TOPIC => Some("Swap(V3)"),
        t if t == SWAP_V2_TOPIC => Some("Swap(V2)"),
        _ => None,
    }
}

pub async fn run(
    ctx: &AppContext,
    address: Option<&str>,
    topic0: Option<&str>,
    participant: Option<&str>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    event: Option<&str>,
) -> Result<LogsResult, EvmError> {
    let latest = ctx.provider.get_block_number().await
        .map_err(|e| EvmError::rpc(format!("get_block_number failed: {e}")))?;

    let end_block = to_block.unwrap_or(latest);
    let start_block = from_block.unwrap_or(end_block.saturating_sub(1000));

    // Resolve topic0 from --event shorthand or --topic0
    let resolved_topic0: Option<B256> = if let Some(ev) = event {
        let topic_hex = resolve_event_topic(ev)?;
        Some(topic_hex.parse().map_err(|_| EvmError::validation("Invalid topic hash"))?)
    } else if let Some(t) = topic0 {
        Some(t.parse().map_err(|_| EvmError::validation(format!("Invalid topic0: {t}")))?)
    } else {
        None
    };

    let mut filter = Filter::new()
        .from_block(start_block)
        .to_block(end_block);

    if let Some(addr_str) = address {
        let addr: Address = addr_str.parse()
            .map_err(|_| EvmError::validation(format!("Invalid address: {addr_str}")))?;
        filter = filter.address(addr);
    }

    if let Some(t0) = resolved_topic0 {
        filter = filter.event_signature(t0);
    }

    // If participant specified, add as topic1 OR topic2 filter
    if let Some(part) = participant {
        let part_addr: Address = part.parse()
            .map_err(|_| EvmError::validation(format!("Invalid participant address: {part}")))?;
        let padded = B256::left_padding_from(part_addr.as_slice());
        // Filter topic1 (from) — we can't OR with topic2 in a single filter,
        // so we filter topic1 and the caller can do a second pass for topic2
        filter = filter.topic1(padded);
    }

    let raw_logs = ctx.provider.get_logs(&filter).await
        .map_err(|e| EvmError::rpc(format!("get_logs failed: {e}")))?;

    let logs: Vec<LogEntry> = raw_logs.iter().map(|log| {
        let topic0_str = log.topics().first()
            .map(|t| format!("{t}"))
            .unwrap_or_default();

        LogEntry {
            address: format!("{}", log.address()),
            block_number: log.block_number.unwrap_or(0),
            tx_hash: log.transaction_hash.map(|h| format!("{h}")).unwrap_or_default(),
            event_name: topic0_to_name(&topic0_str).map(|s| s.to_string()),
            topic0: topic0_str,
            topics: log.topics().iter().skip(1).map(|t| format!("{t}")).collect(),
            data_hex: format!("0x{}", hex::encode(log.data().data.as_ref())),
            log_index: log.log_index.unwrap_or(0) as u32,
        }
    }).collect();

    Ok(LogsResult {
        log_count: logs.len(),
        from_block: start_block,
        to_block: end_block,
        logs,
        rpc_endpoint: ctx.rpc_url.clone(),
    })
}
