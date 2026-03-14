use alloy::primitives::Address;
use alloy::providers::Provider;
use comfy_table::Table;
use serde::Serialize;

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct CodeResult {
    pub address: String,
    pub is_contract: bool,
    pub code_size: usize,
    pub code_hash: Option<String>,
    pub rpc_endpoint: String,
}

impl Tableable for CodeResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.add_row(vec!["Address", &self.address]);
        table.add_row(vec!["Type", if self.is_contract { "Contract" } else { "EOA" }]);
        table.add_row(vec!["Code Size", &format!("{} bytes", self.code_size)]);
        if let Some(ref hash) = self.code_hash {
            table.add_row(vec!["Code Hash", hash]);
        }
        table.add_row(vec!["RPC", &self.rpc_endpoint]);
        table
    }
}

pub async fn run(ctx: &AppContext, address: &str) -> Result<CodeResult, EvmError> {
    let addr: Address = address.parse()
        .map_err(|_| EvmError::validation(format!("Invalid address: {address}")))?;

    let code = ctx.provider.get_code_at(addr).await
        .map_err(|e| EvmError::rpc(format!("get_code failed: {e}")))?;

    let is_contract = !code.is_empty();
    let code_hash = if is_contract {
        Some(format!("{}", alloy::primitives::keccak256(&code)))
    } else {
        None
    };

    Ok(CodeResult {
        address: format!("{addr}"),
        is_contract,
        code_size: code.len(),
        code_hash,
        rpc_endpoint: ctx.rpc_url.clone(),
    })
}
