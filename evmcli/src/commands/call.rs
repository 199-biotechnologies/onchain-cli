use alloy::dyn_abi::{DynSolType, DynSolValue};
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

/// Parse a function signature like "owner()(address)" or "balanceOf(address)(uint256)"
/// Returns (full_input_sig, input_param_types, output_param_types)
fn parse_sig(sig: &str) -> Result<(String, Vec<String>, Vec<String>), EvmError> {
    // Find the split point: ")(" separating input from output
    let (input_sig, output_types_str) = if let Some(pos) = sig.find(")(") {
        let input = &sig[..=pos]; // e.g. "owner()" or "balanceOf(address)"
        let output = &sig[pos+2..sig.len()-1]; // e.g. "address" or "uint256"
        (input.to_string(), output.to_string())
    } else {
        (sig.to_string(), String::new())
    };

    // Parse input params from "funcName(type1,type2)"
    let input_params = if let Some(start) = input_sig.find('(') {
        let params_str = &input_sig[start+1..input_sig.len()-1];
        if params_str.is_empty() {
            vec![]
        } else {
            params_str.split(',').map(|s| s.trim().to_string()).collect()
        }
    } else {
        vec![]
    };

    let output_params = if output_types_str.is_empty() {
        vec![]
    } else {
        output_types_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    Ok((input_sig, input_params, output_params))
}

fn encode_arg(type_str: &str, value: &str) -> Result<DynSolValue, EvmError> {
    let ty: DynSolType = type_str.parse()
        .map_err(|e| EvmError::validation(format!("Invalid type '{type_str}': {e}")))?;

    ty.coerce_str(value)
        .map_err(|e| EvmError::validation(format!("Cannot encode '{value}' as {type_str}: {e}")))
}

pub async fn run(ctx: &AppContext, address: &str, sig: &str, args: &[String]) -> Result<CallResult, EvmError> {
    let addr: Address = address.parse()
        .map_err(|_| EvmError::validation(format!("Invalid address: {address}")))?;

    let (input_sig, input_types, output_types) = parse_sig(sig)?;

    if input_types.len() != args.len() {
        return Err(EvmError::validation(format!(
            "Expected {} args for '{}', got {}",
            input_types.len(), input_sig, args.len()
        )));
    }

    // Build calldata: selector + ABI-encoded args
    let selector = &alloy::primitives::keccak256(input_sig.as_bytes())[..4];
    let mut calldata = selector.to_vec();

    if !args.is_empty() {
        let encoded_args: Vec<DynSolValue> = input_types.iter()
            .zip(args.iter())
            .map(|(t, v)| encode_arg(t, v))
            .collect::<Result<Vec<_>, _>>()?;

        let encoded = DynSolValue::Tuple(encoded_args).abi_encode_params();
        calldata.extend_from_slice(&encoded);
    }

    let tx = alloy::rpc::types::TransactionRequest::default()
        .to(addr)
        .input(alloy::primitives::Bytes::from(calldata).into());

    let result = ctx.provider.call(tx).await
        .map_err(|e| EvmError::rpc(format!("eth_call failed: {e}")))?;

    let result_hex = format!("0x{}", hex::encode(&result));

    // Decode output using dyn-abi
    let decoded = if !output_types.is_empty() {
        decode_output(&result, &output_types)
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

fn decode_output(data: &[u8], types: &[String]) -> Option<String> {
    if data.is_empty() {
        return None;
    }

    let sol_types: Vec<DynSolType> = types.iter()
        .filter_map(|t| t.parse::<DynSolType>().ok())
        .collect();

    if sol_types.len() != types.len() {
        return None;
    }

    if sol_types.len() == 1 {
        let decoded = sol_types[0].abi_decode(data).ok()?;
        Some(format!("{decoded:?}"))
    } else {
        let tuple_type = DynSolType::Tuple(sol_types);
        let decoded = tuple_type.abi_decode_params(data).ok()?;
        Some(format!("{decoded:?}"))
    }
}
