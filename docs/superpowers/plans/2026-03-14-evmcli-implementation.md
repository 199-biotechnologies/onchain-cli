# evmcli Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a fast Rust CLI (`evmcli`) for EVM blockchain operations that replaces the slow MCP server, targeting <500ms per operation on public RPC.

**Architecture:** Single Rust crate (lib + bin). Alloy for RPC, clap for CLI, reqwest for HTTP APIs. Happy-eyeballs RPC auto-detect with disk caching. JSON + table output modes.

**Tech Stack:** Rust, Alloy 1.7, Tokio, clap 4, reqwest 0.12, serde, comfy-table, thiserror, miette

**Spec:** `docs/superpowers/specs/2026-03-14-evmcli-design.md`

**Live test wallet:** `0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A` (Arbitrum One)
**Live test RPC:** `https://arb1.arbitrum.io/rpc` (chainId 42161)

---

## Chunk 1: Foundation (Scaffold + RPC + Balance)

The minimum viable binary: parse CLI args, detect RPC, fetch a balance, output JSON/table.

### Task 1: Scaffold Cargo project

**Files:**
- Create: `evmcli/Cargo.toml`
- Create: `evmcli/src/main.rs`
- Create: `evmcli/src/lib.rs`

- [ ] **Step 1: Create project directory and Cargo.toml**

```bash
mkdir -p ~/Projects/crypto-skill/evmcli/src
```

Write `evmcli/Cargo.toml`:
```toml
[package]
name = "evmcli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "evmcli"
path = "src/main.rs"

[lib]
name = "evmcli"
path = "src/lib.rs"

[dependencies]
alloy = { version = "1.7", features = ["providers", "contract", "reqwest", "dyn-abi", "json-abi", "network", "ens"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
clap = { version = "4", features = ["derive", "env"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
comfy-table = "7"
thiserror = "2"
miette = { version = "7", features = ["fancy"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
directories = "5"
secrecy = { version = "0.10", features = ["serde"] }
toml = "0.8"

[dev-dependencies]
wiremock = "0.6"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 2: Write minimal main.rs and lib.rs that compile**

`evmcli/src/lib.rs`:
```rust
pub mod cli;
pub mod config;
pub mod context;
pub mod errors;
pub mod rpc;
pub mod commands;
pub mod output;
```

`evmcli/src/main.rs`:
```rust
use clap::Parser;
use evmcli::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    println!("evmcli v{}", env!("CARGO_PKG_VERSION"));
}
```

Create stub modules so it compiles:
- `src/cli.rs` with `Cli` struct (clap derive, just `--version` for now)
- `src/config.rs`, `src/context.rs`, `src/errors.rs` as empty `pub mod` stubs
- `src/rpc/mod.rs`, `src/commands/mod.rs`, `src/output/mod.rs` as empty stubs

- [ ] **Step 3: Verify it compiles and runs**

```bash
cd ~/Projects/crypto-skill/evmcli && cargo build 2>&1 | tail -5
./target/debug/evmcli --version
```
Expected: prints version `evmcli 0.1.0`

- [ ] **Step 4: Commit**

```bash
git add evmcli/ && git commit -m "feat(evmcli): scaffold Rust project with Alloy + clap"
```

---

### Task 2: Error types and output rendering

**Files:**
- Create: `evmcli/src/errors.rs`
- Create: `evmcli/src/output/mod.rs`
- Create: `evmcli/src/output/json.rs`
- Create: `evmcli/src/output/table.rs`

- [ ] **Step 1: Write EvmError enum**

`evmcli/src/errors.rs`:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvmError {
    #[error("RPC error: {message}")]
    Rpc { code: &'static str, message: String },

    #[error("Explorer API error: {message}")]
    Explorer { code: &'static str, message: String },

    #[error("ABI error: {message}")]
    Abi { code: &'static str, message: String },

    #[error("Decode error: {message}")]
    Decode { code: &'static str, message: String },

    #[error("Signing error: {message}")]
    Signing { code: &'static str, message: String },

    #[error("Validation error: {message}")]
    Validation { code: &'static str, message: String },

    #[error("Config error: {message}")]
    Config { code: &'static str, message: String },
}

impl EvmError {
    pub fn machine_code(&self) -> &'static str {
        match self {
            Self::Rpc { code, .. } => code,
            Self::Explorer { code, .. } => code,
            Self::Abi { code, .. } => code,
            Self::Decode { code, .. } => code,
            Self::Signing { code, .. } => code,
            Self::Validation { code, .. } => code,
            Self::Config { code, .. } => code,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config { .. } => 2,
            Self::Rpc { .. } => 3,
            Self::Signing { .. } => 5,
            _ => 1,
        }
    }
}
```

- [ ] **Step 2: Write output module with JSON and table rendering**

`evmcli/src/output/mod.rs`:
```rust
pub mod json;
pub mod table;

use serde::Serialize;
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputFormat {
    Json,
    Table,
}

impl OutputFormat {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Self::Json
        } else {
            Self::Table
        }
    }
}

pub fn render<T: Serialize + table::Tableable>(value: &T, format: OutputFormat) {
    match format {
        OutputFormat::Json => json::render(value),
        OutputFormat::Table => table::render(value),
    }
}

pub fn render_error(err: &crate::errors::EvmError, format: OutputFormat) {
    match format {
        OutputFormat::Json => json::render_error(err),
        OutputFormat::Table => {
            eprintln!("Error: {err}");
        }
    }
}
```

`evmcli/src/output/json.rs`:
```rust
use serde::Serialize;
use crate::errors::EvmError;

pub fn render<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

pub fn render_error(err: &EvmError) {
    let json = serde_json::json!({
        "error": err.machine_code(),
        "message": err.to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

`evmcli/src/output/table.rs`:
```rust
use serde::Serialize;

pub trait Tableable {
    fn to_table(&self) -> comfy_table::Table;
}

pub fn render<T: Tableable>(value: &T) {
    println!("{}", value.to_table());
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd ~/Projects/crypto-skill/evmcli && cargo check 2>&1 | head -5
```

- [ ] **Step 4: Commit**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): add error types and output rendering (json + table)"
```

---

### Task 3: Chain config and RPC auto-detect

**Files:**
- Create: `evmcli/src/config.rs`
- Create: `evmcli/src/rpc/mod.rs`
- Create: `evmcli/src/rpc/detect.rs`
- Create: `evmcli/src/rpc/provider.rs`

- [ ] **Step 1: Write chain config with baked-in defaults**

`evmcli/src/config.rs` — define `ChainConfig` struct with name, chain_id, rpc_url, explorer_url, local_rpc for known chains. Implement `Default` returning Arbitrum. Support `--network` flag to select chain. Load `EVMCLI_RPC_URL` env override.

Key chains: arbitrum (42161), ethereum (1), base (8453), optimism (10), polygon (137).

- [ ] **Step 2: Write RPC auto-detect with happy-eyeballs probing**

`evmcli/src/rpc/detect.rs`:
- `EndpointSelector` struct
- `select_endpoint()` async fn:
  1. Check `--rpc-url` override → return immediately
  2. Check disk cache at `~/.cache/evmcli/rpc_winner` (if <30s old) → return cached
  3. Spawn two probes concurrently:
     - Local probe: `tokio::time::timeout(200ms, eth_chainId on localhost:8547)` starts immediately
     - Public probe: starts after 40ms delay
     - First probe returning correct chainId wins
  4. Write winner to disk cache
- Use `reqwest::Client` directly for probing (raw JSON-RPC POST), not Alloy provider (too heavy to construct just to probe)

- [ ] **Step 3: Write provider builder**

`evmcli/src/rpc/provider.rs`:
- `build_read_provider(rpc_url: &str)` → Alloy provider with reqwest transport, no wallet
- `build_write_provider(rpc_url: &str, private_key: &str)` → Alloy provider with wallet + gas/nonce fillers
- Shared `reqwest::Client` with `pool_idle_timeout(60s)`, `tcp_nodelay(true)`, `timeout(10s)`

- [ ] **Step 4: Live test — probe RPC and verify chainId**

```bash
cd ~/Projects/crypto-skill/evmcli && cargo run -- --help
# Then test with a simple probe (we'll wire this up properly in balance command)
```

- [ ] **Step 5: Commit**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): chain config + happy-eyeballs RPC auto-detect"
```

---

### Task 4: Balance command (first real command)

**Files:**
- Create: `evmcli/src/commands/mod.rs`
- Create: `evmcli/src/commands/balance.rs`
- Create: `evmcli/src/context.rs`
- Modify: `evmcli/src/cli.rs` — add `Balance` subcommand
- Modify: `evmcli/src/main.rs` — dispatch to balance

- [ ] **Step 1: Write AppContext**

`evmcli/src/context.rs`:
- `AppContext` struct: holds Alloy provider, reqwest client, chain config, output format
- `AppContext::new(cli: &Cli)` → runs RPC detect, builds provider, returns context

- [ ] **Step 2: Write CLI with Balance subcommand**

`evmcli/src/cli.rs`:
```rust
#[derive(Parser)]
#[command(name = "evmcli", version, about = "Fast EVM CLI toolkit")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format: json or table (auto-detects TTY)
    #[arg(long, global = true)]
    pub json: bool,

    /// Network name or chain ID
    #[arg(long, global = true, default_value = "arbitrum", env = "EVMCLI_NETWORK")]
    pub network: String,

    /// Custom RPC URL (overrides auto-detect)
    #[arg(long, global = true, env = "EVMCLI_RPC_URL")]
    pub rpc_url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get native token balance
    Balance {
        /// Address to check
        address: String,
        /// ERC20 token contract address
        #[arg(long)]
        token: Option<String>,
    },
}
```

- [ ] **Step 3: Write balance command**

`evmcli/src/commands/balance.rs`:
- `BalanceResult` struct (serde Serialize + Tableable)
- `run(ctx: &AppContext, address: &str, token: Option<&str>) -> Result<BalanceResult, EvmError>`
- ETH balance: `provider.get_balance(address).await`
- ERC20 balance: `sol!` ERC20 balanceOf call
- Format to ETH (divide by 1e18) and include raw wei

- [ ] **Step 4: Wire up main.rs dispatch**

```rust
match cli.command {
    Commands::Balance { address, token } => {
        let result = commands::balance::run(&ctx, &address, token.as_deref()).await?;
        output::render(&result, format);
    }
}
```

- [ ] **Step 5: Live test against real Arbitrum**

```bash
cd ~/Projects/crypto-skill/evmcli
# ETH balance
cargo run -- balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
# Same in JSON
cargo run -- --json balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
# Compare with cast
cast balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A --rpc-url https://arb1.arbitrum.io/rpc -e
```

Verify outputs match. Time both — evmcli must be <500ms.

- [ ] **Step 6: Benchmark against cast**

```bash
# Benchmark evmcli
time cargo run --release -- --json balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
# Benchmark cast
time cast balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A --rpc-url https://arb1.arbitrum.io/rpc -e
```

Record results. If evmcli is slower than cast, investigate and fix before proceeding.

- [ ] **Step 7: Commit**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): balance command — live tested, benchmarked vs cast"
```

---

## Chunk 2: Core Read Commands (tx, receipt, block, gas)

### Task 5: Transaction command

**Files:**
- Create: `evmcli/src/commands/tx.rs`
- Modify: `evmcli/src/cli.rs` — add `Tx` subcommand

- [ ] **Step 1: Write TxResult struct + run function**

Fetch transaction by hash via `provider.get_transaction_by_hash()`. Return block, from, to, value, gas, input data.

- [ ] **Step 2: Wire CLI, live test, benchmark**

```bash
# Get a recent TX hash from Blockscout
TXHASH=$(curl -s "https://arbitrum.blockscout.com/api/v2/addresses/0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A/transactions" | python3 -c "import sys,json; print(json.load(sys.stdin)['items'][0]['hash'])")
cargo run --release -- tx $TXHASH
time cargo run --release -- --json tx $TXHASH
time cast tx $TXHASH --rpc-url https://arb1.arbitrum.io/rpc
```

- [ ] **Step 3: Commit**

### Task 6: Receipt command

**Files:**
- Create: `evmcli/src/commands/receipt.rs`

- [ ] **Step 1: Write receipt command**

Fetch receipt via `provider.get_transaction_receipt()`. Show status, gas used, logs count.

- [ ] **Step 2: Live test + benchmark vs cast receipt**

- [ ] **Step 3: Commit**

### Task 7: Block command

**Files:**
- Create: `evmcli/src/commands/block.rs`

- [ ] **Step 1: Write block command**

Accept `latest`, block number, or block hash. Fetch via provider.

- [ ] **Step 2: Live test + benchmark**

- [ ] **Step 3: Commit**

### Task 8: Gas command

**Files:**
- Create: `evmcli/src/commands/gas.rs`

- [ ] **Step 1: Write gas command**

Show current gas price, EIP-1559 base fee + priority fee. Format in gwei.

- [ ] **Step 2: Live test**

- [ ] **Step 3: Commit all Chunk 2**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): tx, receipt, block, gas commands — all live tested"
```

---

## Chunk 3: Explorer + ABI + Decode

### Task 9: Blockscout explorer client

**Files:**
- Create: `evmcli/src/explorer/mod.rs`
- Create: `evmcli/src/explorer/blockscout.rs`
- Create: `evmcli/src/explorer/models.rs`
- Create: `evmcli/src/commands/explorer.rs`

- [ ] **Step 1: Write typed Blockscout v2 client**

`BlockscoutClient` struct with `reqwest::Client`. Methods:
- `get_transactions(address, limit)` → `Vec<BlockscoutTx>`
- `get_token_transfers(address)` → `Vec<TokenTransfer>`
- Typed response models in `models.rs`

- [ ] **Step 2: Write `txs` CLI command**

```bash
cargo run --release -- txs 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
time cargo run --release -- --json txs 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
time curl -s "https://arbitrum.blockscout.com/api/v2/addresses/0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A/transactions" | python3 -c "import sys; print(len(sys.stdin.read()))"
```

Must match curl speed (<200ms).

- [ ] **Step 3: Commit**

### Task 10: ABI fetch + cache

**Files:**
- Create: `evmcli/src/abi/mod.rs`
- Create: `evmcli/src/abi/cache.rs`
- Create: `evmcli/src/abi/fetch.rs`
- Create: `evmcli/src/commands/abi.rs`

- [ ] **Step 1: Write ABI disk cache**

Cache at `~/.cache/evmcli/abis/{chainId}/{address}.json`. Read/write with serde.

- [ ] **Step 2: Write ABI fetch cascade**

Blockscout `/api?module=contract&action=getabi` → Etherscan fallback (if key set).

- [ ] **Step 3: Write `abi` CLI command + live test**

```bash
# Fetch ABI for Aave V3 Pool on Arbitrum
cargo run --release -- abi 0x794a61358D6845594F94dc1DB02A252b5b4814aD
# Second run should be instant (cached)
time cargo run --release -- --json abi 0x794a61358D6845594F94dc1DB02A252b5b4814aD
```

- [ ] **Step 4: Commit**

### Task 11: Calldata decode

**Files:**
- Create: `evmcli/src/abi/decode.rs`
- Create: `evmcli/src/abi/known.rs`
- Create: `evmcli/src/commands/decode.rs`

- [ ] **Step 1: Write baked-in known selectors**

`known.rs`: use `sol!` macro to define ERC20, Uniswap V3, Aave V3, Balancer V2 function signatures. Build a `HashMap<[u8;4], &'static str>` of selector → human-readable name.

- [ ] **Step 2: Write decode logic**

1. Check baked-in selectors first
2. If not found, query `sig.eth.samczsun.com` API
3. If ABI cached for the target contract, decode full args

- [ ] **Step 3: Write `decode` CLI command + live test**

```bash
# Decode an ERC20 transfer
cargo run --release -- decode 0xa9059cbb0000000000000000000000004a0acac60321d89e8d4d01fa09318849cb6a586a0000000000000000000000000000000000000000000000000000000005f5e100
# Should show: transfer(address,uint256)
```

- [ ] **Step 4: Commit all Chunk 3**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): explorer, ABI cache, calldata decode — all live tested"
```

---

## Chunk 4: Contract Read/Write + Transfer + Approve

### Task 12: Contract read (eth_call)

**Files:**
- Create: `evmcli/src/commands/contract.rs`
- Create: `evmcli/src/contracts/erc20.rs`
- Create: `evmcli/src/contracts/multicall3.rs`

- [ ] **Step 1: Write contract read command**

Accept `address`, function signature string (like `"owner()(address)"`), optional args. Parse with `alloy-dyn-abi`. Execute `eth_call`.

- [ ] **Step 2: Live test**

```bash
# Read owner of MoonshotExecutorV3
cargo run --release -- call 0x33643b0c0C9bc97a8aa9dAe7D894B3a63192e3D2 "owner()(address)"
# Compare with cast
cast call 0x33643b0c0C9bc97a8aa9dAe7D894B3a63192e3D2 "owner()(address)" --rpc-url https://arb1.arbitrum.io/rpc
```

- [ ] **Step 3: Commit**

### Task 13: Transfer ETH + ERC20

**Files:**
- Create: `evmcli/src/commands/transfer.rs`

- [ ] **Step 1: Write transfer command**

Requires `WriteProvider` (wallet loaded). ETH transfer: `provider.send_transaction()`. ERC20 transfer: `sol!` ERC20 transfer call.

Safety: print preview (from, to, amount, estimated gas) and wait 3s before sending. `--yes` flag to skip confirmation.

- [ ] **Step 2: Test with tiny amount (0.0001 ETH) from the live wallet**

```bash
# DRY RUN — just show what would happen
cargo run --release -- transfer 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A 0.0001 --dry-run
```

- [ ] **Step 3: Commit**

### Task 14: Approve command

**Files:**
- Create: `evmcli/src/commands/approve.rs`

- [ ] **Step 1: Write approve command**

ERC20 approve via `sol!` binding. Same safety pattern as transfer.

- [ ] **Step 2: Commit Chunk 4**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): contract read/write, transfer, approve — live tested"
```

---

## Chunk 5: ENS + Bench + Status

### Task 15: ENS resolution

**Files:**
- Create: `evmcli/src/commands/ens.rs`

- [ ] **Step 1: Write ENS command with separate L1 provider**

ENS resolution starts on Ethereum mainnet. Create a separate provider pointing at `eth.llamarpc.com`. Resolve name → address and address → name.

- [ ] **Step 2: Live test**

```bash
cargo run --release -- ens resolve vitalik.eth
# Should return 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

- [ ] **Step 3: Commit**

### Task 16: Benchmark command

**Files:**
- Create: `evmcli/src/commands/bench.rs`

- [ ] **Step 1: Write bench command**

Run N iterations of balance/tx/receipt/block. Collect timings. Compute mean, p50, p95, p99, min, max. Show which RPC endpoint was selected. Optional `--operation` filter. `--warmup` flag.

- [ ] **Step 2: Live test**

```bash
cargo run --release -- bench --iterations 20 --warmup 3 --address 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
```

Verify all p95 < 500ms on public RPC.

- [ ] **Step 3: Commit**

### Task 17: Status command (replaces quick-check.sh)

**Files:**
- Create: `evmcli/src/commands/status.rs`

- [ ] **Step 1: Write status command**

Parallel `tokio::join!` of:
- Balance of configured wallets (from config or CLI args)
- Last N transactions from Blockscout
- Contract latest activity

Configurable via `~/.config/evmcli/status.toml` or `--address` flags.

Default addresses: main wallet `0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A`, competitor `0x00000297dbF14d9DEb0904f88034D5C5e46C6f2f`.

- [ ] **Step 2: Live test + benchmark vs quick-check.sh**

```bash
time cargo run --release -- status
time ops/quick-check.sh  # from moonshot-crypto dir
```

evmcli status must be <= quick-check.sh time (0.8s target).

- [ ] **Step 3: Commit Chunk 5**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): ENS, benchmark, status command — all live benchmarked"
```

---

## Chunk 6: Polish + Install + Final Benchmarks

### Task 18: Release build + install + comprehensive benchmark

- [ ] **Step 1: Build release binary**

```bash
cd ~/Projects/crypto-skill/evmcli
cargo build --release
ls -la target/release/evmcli
```

Note binary size.

- [ ] **Step 2: Install to PATH**

```bash
mkdir -p ~/.local/bin
ln -sf ~/Projects/crypto-skill/evmcli/target/release/evmcli ~/.local/bin/evmcli
evmcli --version
```

- [ ] **Step 3: Run full benchmark suite**

```bash
evmcli bench --iterations 50 --warmup 5 --address 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
```

Compare every operation vs:
- `cast` (same RPC)
- `mcp__evm__*` (from the session benchmarks: 3-8s)
- `ops/quick-check.sh` (0.8s)

All operations must meet targets: <500ms public, <100ms local (if tunnel up).

- [ ] **Step 4: Run all commands once to verify correctness**

```bash
evmcli balance 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
evmcli tx $(curl -s "https://arbitrum.blockscout.com/api/v2/addresses/0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A/transactions" | python3 -c "import sys,json; print(json.load(sys.stdin)['items'][0]['hash'])")
evmcli gas
evmcli txs 0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A
evmcli decode 0xa9059cbb0000000000000000000000004a0acac60321d89e8d4d01fa09318849cb6a586a0000000000000000000000000000000000000000000000000000000005f5e100
evmcli status
evmcli bench --iterations 10
```

All must succeed with correct output.

- [ ] **Step 5: Final commit + push**

```bash
git add -A evmcli/ && git commit -m "feat(evmcli): v0.1.0 — complete CLI with benchmarks, replaces MCP server"
git push
```

---

## Post-Implementation

After all chunks complete:

1. **Update moonshot-ops skill** to reference `evmcli` commands instead of `cast`/`curl` where beneficial
2. **Consider removing `@mcpdotdirect/evm-mcp-server`** from Claude Code MCP config (`claude mcp remove evm`)
3. **Create `evmcli` Claude Code skill** documenting all commands for future sessions
4. **Save benchmark results** as a feedback memory for future performance comparisons
