use clap::Parser;
use evmcli::cli::{Cli, Commands};
use evmcli::context::AppContext;
use evmcli::output::{self, OutputFormat};
use std::process;
use std::time::Instant;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let format = OutputFormat::detect(cli.json);

    let start = Instant::now();

    let ctx = match AppContext::new(&cli).await {
        Ok(ctx) => ctx,
        Err(e) => {
            output::render_error(&e, format);
            process::exit(e.exit_code());
        }
    };

    let result = match cli.command {
        Commands::Balance { ref address, ref token } => {
            evmcli::commands::balance::run(&ctx, address, token.as_deref()).await
                .map(|r| output::render(&r, format))
        }
        Commands::Tx { ref hash } => {
            evmcli::commands::tx::run(&ctx, hash).await
                .map(|r| output::render(&r, format))
        }
        Commands::Receipt { ref hash } => {
            evmcli::commands::receipt::run(&ctx, hash).await
                .map(|r| output::render(&r, format))
        }
        Commands::Block { ref id } => {
            evmcli::commands::block::run(&ctx, id).await
                .map(|r| output::render(&r, format))
        }
        Commands::Gas => {
            evmcli::commands::gas::run(&ctx).await
                .map(|r| output::render(&r, format))
        }
        Commands::Call { ref address, ref sig, ref args } => {
            evmcli::commands::call::run(&ctx, address, sig, args).await
                .map(|r| output::render(&r, format))
        }
        Commands::Txs { ref address } => {
            evmcli::commands::explorer::run(&ctx, address).await
                .map(|r| output::render(&r, format))
        }
        Commands::Decode { ref data } => {
            evmcli::commands::decode::run(&ctx, data).await
                .map(|r| output::render(&r, format))
        }
        Commands::Abi { ref address } => {
            evmcli::commands::abi::run(&ctx, address).await
                .map(|r| output::render(&r, format))
        }
        Commands::Logs { ref address, ref topic0, ref participant, from_block, to_block, ref event } => {
            evmcli::commands::logs::run(&ctx,
                address.as_deref(), topic0.as_deref(), participant.as_deref(),
                from_block, to_block, event.as_deref(),
            ).await.map(|r| output::render(&r, format))
        }
        Commands::Transfers { ref address, ref token_type } => {
            evmcli::commands::transfers::run(&ctx, address, token_type).await
                .map(|r| output::render(&r, format))
        }
        Commands::Storage { ref address, ref slot, block } => {
            evmcli::commands::storage::run(&ctx, address, slot, block).await
                .map(|r| output::render(&r, format))
        }
        Commands::Nonce { ref address } => {
            evmcli::commands::nonce::run(&ctx, address).await
                .map(|r| output::render(&r, format))
        }
        Commands::Code { ref address } => {
            evmcli::commands::code::run(&ctx, address).await
                .map(|r| output::render(&r, format))
        }
        Commands::Trace { ref hash } => {
            evmcli::commands::trace::run(&ctx, hash).await
                .map(|r| output::render(&r, format))
        }
        Commands::Bench { iterations, warmup, ref address } => {
            evmcli::commands::bench::run(&ctx, iterations, warmup, address).await
                .map(|r| output::render(&r, format))
        }
    };

    let elapsed = start.elapsed();
    tracing::debug!("Completed in {:.3}s", elapsed.as_secs_f64());

    if let Err(e) = result {
        output::render_error(&e, format);
        process::exit(e.exit_code());
    }
}
