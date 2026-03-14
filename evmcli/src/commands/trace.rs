use alloy::primitives::B256;
use comfy_table::Table;
use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct TraceResult {
    pub hash: String,
    pub call_count: usize,
    pub calls: Vec<TraceCall>,
    pub rpc_endpoint: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TraceCall {
    pub depth: usize,
    pub call_type: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas_used: String,
    pub input_size: usize,
    pub output_size: usize,
    pub error: Option<String>,
}

impl Tableable for TraceResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Depth", "Type", "From", "To", "Value", "Gas", "Error"]);
        for call in &self.calls {
            let indent = "  ".repeat(call.depth);
            let from_short = if call.from.len() > 14 {
                format!("{}...{}", &call.from[..8], &call.from[call.from.len()-4..])
            } else { call.from.clone() };
            let to_short = if call.to.len() > 14 {
                format!("{}...{}", &call.to[..8], &call.to[call.to.len()-4..])
            } else { call.to.clone() };
            table.add_row(vec![
                &format!("{indent}{}", call.depth),
                &call.call_type,
                &from_short,
                &to_short,
                &call.value,
                &call.gas_used,
                call.error.as_deref().unwrap_or(""),
            ]);
        }
        table.add_row(vec![&format!("{} calls", self.call_count), "", "", "", "", "", ""]);
        table
    }
}

// The debug_traceTransaction response structure
#[derive(Debug, Deserialize)]
struct DebugTraceResponse {
    result: Option<TraceFrame>,
}

#[derive(Debug, Deserialize)]
struct TraceFrame {
    #[serde(rename = "type")]
    call_type: Option<String>,
    from: Option<String>,
    to: Option<String>,
    value: Option<String>,
    gas: Option<String>,
    #[serde(rename = "gasUsed")]
    gas_used: Option<String>,
    input: Option<String>,
    output: Option<String>,
    error: Option<String>,
    calls: Option<Vec<TraceFrame>>,
}

fn flatten_calls(frame: &TraceFrame, depth: usize, result: &mut Vec<TraceCall>) {
    result.push(TraceCall {
        depth,
        call_type: frame.call_type.clone().unwrap_or_else(|| "CALL".to_string()),
        from: frame.from.clone().unwrap_or_default(),
        to: frame.to.clone().unwrap_or_default(),
        value: frame.value.clone().unwrap_or_else(|| "0x0".to_string()),
        gas_used: frame.gas_used.clone().unwrap_or_else(|| "0".to_string()),
        input_size: frame.input.as_ref().map(|i| (i.len().saturating_sub(2)) / 2).unwrap_or(0),
        output_size: frame.output.as_ref().map(|o| (o.len().saturating_sub(2)) / 2).unwrap_or(0),
        error: frame.error.clone(),
    });

    if let Some(ref subcalls) = frame.calls {
        for subcall in subcalls {
            flatten_calls(subcall, depth + 1, result);
        }
    }
}

pub async fn run(ctx: &AppContext, hash: &str) -> Result<TraceResult, EvmError> {
    let _tx_hash: B256 = hash.parse()
        .map_err(|_| EvmError::validation(format!("Invalid tx hash: {hash}")))?;

    // Use raw JSON-RPC for debug_traceTransaction (Alloy doesn't have a typed method for this)
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "debug_traceTransaction",
        "params": [hash, {"tracer": "callTracer", "tracerConfig": {"onlyTopCall": false}}],
        "id": 1
    });

    let resp = ctx.http.post(&ctx.rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| EvmError::rpc(format!("debug_traceTransaction request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(EvmError::rpc(format!(
            "debug_traceTransaction returned HTTP {}. This requires an archive node with debug API enabled.",
            resp.status()
        )));
    }

    let trace_resp: serde_json::Value = resp.json().await
        .map_err(|e| EvmError::rpc(format!("Failed to parse trace response: {e}")))?;

    // Check for JSON-RPC error
    if let Some(error) = trace_resp.get("error") {
        let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
        return Err(EvmError::Rpc {
            code: "rpc.trace_unsupported",
            message: format!("debug_traceTransaction failed: {msg}. This requires an archive node (local node or paid RPC like Alchemy/QuickNode)."),
        });
    }

    let frame: TraceFrame = serde_json::from_value(
        trace_resp.get("result").cloned().unwrap_or(serde_json::Value::Null)
    ).map_err(|e| EvmError::rpc(format!("Failed to parse trace frame: {e}")))?;

    let mut calls = Vec::new();
    flatten_calls(&frame, 0, &mut calls);

    Ok(TraceResult {
        hash: hash.to_string(),
        call_count: calls.len(),
        calls,
        rpc_endpoint: ctx.rpc_url.clone(),
    })
}
