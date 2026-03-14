use alloy::providers::Provider;
use comfy_table::Table;
use serde::Serialize;

use crate::context::AppContext;
use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct GasResult {
    pub gas_price_gwei: f64,
    pub base_fee_gwei: Option<f64>,
    pub rpc_endpoint: String,
}

impl Tableable for GasResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.add_row(vec!["Gas Price", &format!("{:.4} gwei", self.gas_price_gwei)]);
        if let Some(base) = self.base_fee_gwei {
            table.add_row(vec!["Base Fee", &format!("{:.4} gwei", base)]);
        }
        table.add_row(vec!["RPC", &self.rpc_endpoint]);
        table
    }
}

pub async fn run(ctx: &AppContext) -> Result<GasResult, EvmError> {
    let gas_price = ctx.provider.get_gas_price().await
        .map_err(|e| EvmError::rpc(format!("get_gas_price failed: {e}")))?;

    let latest_block = ctx.provider.get_block_by_number(alloy::eips::BlockNumberOrTag::Latest).await
        .map_err(|e| EvmError::rpc(format!("get_block failed: {e}")))?;

    let base_fee = latest_block.and_then(|b| b.header.base_fee_per_gas);

    Ok(GasResult {
        gas_price_gwei: gas_price as f64 / 1e9,
        base_fee_gwei: base_fee.map(|f| f as f64 / 1e9),
        rpc_endpoint: ctx.rpc_url.clone(),
    })
}
