use alloy::primitives::Address;
use alloy::providers::Provider;
use comfy_table::Table;
use serde::Serialize;

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct CallResult {
    pub contract: String,
    pub function: String,
    pub result_hex: String,
    pub result_decoded: Option<String>,
    pub rpc_endpoint: String,
}

impl Tableable for CallResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.add_row(vec!["Contract", &self.contract]);
        table.add_row(vec!["Function", &self.function]);
        if let Some(ref decoded) = self.result_decoded {
            table.add_row(vec!["Result", decoded]);
        }
        table.add_row(vec!["Raw", &self.result_hex]);
        table.add_row(vec!["RPC", &self.rpc_endpoint]);
        table
    }
}

pub async fn run(ctx: &AppContext, address: &str, sig: &str, _args: &[String]) -> Result<CallResult, EvmError> {
    let addr: Address = address.parse()
        .map_err(|_| EvmError::validation(format!("Invalid address: {address}")))?;

    // Parse function signature like "owner()(address)" or "balanceOf(address)(uint256)"
    // Split on ")(" to get input sig and output types
    let (func_sig, output_types) = if let Some(pos) = sig.find(")(") {
        let input_sig = &sig[..=pos]; // e.g. "owner()"
        let output = &sig[pos+1..];  // e.g. "(address)"
        (input_sig.to_string(), Some(output.to_string()))
    } else {
        (sig.to_string(), None)
    };

    // Encode the function call using keccak256 of the signature
    let selector = alloy::primitives::keccak256(func_sig.as_bytes());
    let calldata = selector[..4].to_vec();

    let tx = alloy::rpc::types::TransactionRequest::default()
        .to(addr)
        .input(alloy::primitives::Bytes::from(calldata).into());

    let result = ctx.provider.call(tx).await
        .map_err(|e| EvmError::rpc(format!("eth_call failed: {e}")))?;

    let result_hex = format!("0x{}", hex::encode(&result));

    // Try to decode simple types
    let decoded = if let Some(ref out_types) = output_types {
        decode_simple_output(&result, out_types)
    } else {
        None
    };

    Ok(CallResult {
        contract: format!("{addr}"),
        function: sig.to_string(),
        result_hex,
        result_decoded: decoded,
        rpc_endpoint: ctx.rpc_url.clone(),
    })
}

fn decode_simple_output(data: &[u8], types: &str) -> Option<String> {
    if data.len() < 32 {
        return None;
    }
    match types.trim() {
        "(address)" => {
            if data.len() >= 32 {
                let addr = Address::from_slice(&data[12..32]);
                Some(format!("{addr}"))
            } else {
                None
            }
        }
        "(uint256)" => {
            let val = alloy::primitives::U256::from_be_slice(data);
            Some(val.to_string())
        }
        "(bool)" => {
            let val = data[31] != 0;
            Some(val.to_string())
        }
        "(uint8)" => {
            Some(data[31].to_string())
        }
        _ => None,
    }
}
