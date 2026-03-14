use crate::cli::Cli;
use crate::config::{self, ChainConfig};
use crate::errors::EvmError;
use crate::output::OutputFormat;
use crate::rpc::{detect, provider};

pub struct AppContext {
    pub provider: provider::ReadProvider,
    pub http: reqwest::Client,
    pub chain: &'static ChainConfig,
    pub format: OutputFormat,
    pub rpc_url: String,
}

impl AppContext {
    pub async fn new(cli: &Cli) -> Result<Self, EvmError> {
        let chain = config::resolve_chain(&cli.network)?;
        let http = provider::build_http_client();

        let rpc_url = detect::select_endpoint(
            cli.rpc_url.as_deref(),
            chain,
            &http,
        ).await?;

        let alloy_provider = provider::build_read_provider(&rpc_url).await?;
        let format = OutputFormat::detect(cli.json);

        Ok(Self {
            provider: alloy_provider,
            http,
            chain,
            format,
            rpc_url,
        })
    }
}
