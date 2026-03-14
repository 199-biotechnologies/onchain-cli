# evmcli — Fast EVM CLI Tool

**Date:** 2026-03-14
**Status:** Approved
**Location:** `/Users/biobook/Projects/crypto-skill/evmcli/`

## Purpose

Replace the slow `@mcpdotdirect/evm-mcp-server` (3-8s per call, hardcoded public RPCs) with a fast Rust CLI that works as both a standalone terminal tool and a Claude Code Bash tool. General-purpose EVM toolkit, not chain-specific — defaults to Arbitrum but supports any EVM chain via `--network` flag.

## Performance Targets

- Local node (SSH tunnel): <100ms per operation
- Public RPC: <500ms per operation
- Blockscout API: <200ms for tx lists
- Must beat `cast` or match it — never slower

## Architecture

Single Rust crate, library + binary target.

### Module Structure

```
evmcli/
  Cargo.toml
  src/
    main.rs           # thin adapter: parse args, build context, dispatch, render
    lib.rs            # re-exports for library use
    cli.rs            # clap derive structs for all subcommands
    config.rs         # env vars, .env file loading, chain configs
    context.rs        # AppContext: provider + http client + config
    errors.rs         # EvmError enum, machine codes, exit codes
    rpc/
      mod.rs
      detect.rs       # happy-eyeballs RPC probing + disk cache
      provider.rs     # ReadProvider (no wallet) / WriteProvider (signing)
    commands/
      balance.rs      # ETH + ERC20 balance, multicall batch for multiple addrs
      tx.rs           # get transaction by hash
      receipt.rs      # get receipt + decode logs
      block.rs        # get block by number/hash/latest
      contract.rs     # read (eth_call) + write (send_tx) contract
      transfer.rs     # send ETH / ERC20
      approve.rs      # ERC20 approve
      gas.rs          # gas price + EIP-1559 base/priority fees
      ens.rs          # resolve/reverse (uses L1 mainnet provider)
      abi.rs          # fetch + cache ABI from explorer
      decode.rs       # calldata decode (baked sigs + 4byte lookup)
      explorer.rs     # Blockscout: tx list, token transfers
      bench.rs        # built-in benchmark with p50/p95/p99
      status.rs       # parallel status check (replaces quick-check.sh)
    explorer/
      blockscout.rs   # typed Blockscout v2 API client
      etherscan.rs    # Etherscan-compatible API (ABI, source, txlist)
      models.rs       # response structs
    abi/
      cache.rs        # disk cache in ~/.cache/evmcli/abis/
      fetch.rs        # Blockscout -> Etherscan fallback cascade
      decode.rs       # dyn-abi for generic + baked sol! for known
      known.rs        # baked-in selectors: ERC20, Uniswap, Aave, Balancer
    output/
      json.rs         # --json: strict JSON stdout, diagnostics stderr
      table.rs        # --table (default TTY): comfy-table with colors
    contracts/        # sol! typed bindings
      erc20.rs
      multicall3.rs
```

### RPC Auto-detect (Happy-Eyeballs)

```
Priority order:
1. --rpc-url flag or EVMCLI_RPC_URL env → use directly
2. Disk cache (~/.cache/evmcli/rpc_winner, <30s old) → reuse winner
3. Parallel probe:
   a. Start localhost:8547 probe immediately
   b. After 40ms if local hasn't responded, fire public RPC probe
   c. Probe = tokio::time::timeout(200ms, eth_chainId)
   d. Verify chainId matches --network (default: 42161 for Arbitrum)
4. Cache winner to disk for 30s
```

### Chain Configuration

Default chain configs baked in. `--network` flag or `EVMCLI_NETWORK` env:

```
arbitrum (default): chainId=42161, rpc=arb1.arbitrum.io/rpc, explorer=arbitrum.blockscout.com
ethereum: chainId=1, rpc=eth.llamarpc.com, explorer=eth.blockscout.com
base: chainId=8453, rpc=mainnet.base.org, explorer=base.blockscout.com
... (extensible via config file)
```

### Provider Split

- **ReadProvider**: no wallet, used for all read commands (balance, tx, receipt, block, contract read, explorer, abi, decode, ens, gas, bench, status)
- **WriteProvider**: loads wallet from `EVMCLI_PRIVATE_KEY` or `PRIVATE_KEY` env, adds gas/nonce fillers. Used only for: transfer, approve, contract write.
- Shared `reqwest::Client` with connection pooling, TCP_NODELAY, 10s timeout.

### Output Modes

Every command returns a serializable DTO struct. Rendered via:
- `--json`: strict JSON on stdout. Nothing else on stdout. Diagnostics/errors on stderr. For Claude Code.
- `--table` (default when TTY): `comfy-table` with ANSI colors. For humans.
- When piped (no TTY): auto-switches to JSON.

### Error Handling

Library: `EvmError` enum with variants: `Rpc`, `Explorer`, `Abi`, `Decode`, `Signing`, `Validation`, `Config`.

Binary: maps to `miette` for TTY, stable JSON errors for `--json`:
```json
{"error": "rpc.timeout", "message": "RPC call timed out after 10s", "details": "eth_getTransactionByHash"}
```

Exit codes: 0=success, 1=usage, 2=config, 3=rpc error, 4=tx reverted, 5=signing error.

### Wallet Configuration

Reads from (in order):
1. `EVMCLI_PRIVATE_KEY` env
2. `PRIVATE_KEY` env (compatible with moonshot-crypto .env)
3. `--private-key` flag (discouraged, visible in process list)

### ABI Caching

Disk cache at `~/.cache/evmcli/abis/{chainId}/{address}.json`. Cascade:
1. Check disk cache (never expires for verified contracts)
2. Blockscout API: `/api?module=contract&action=getabi`
3. Etherscan V2 API fallback (if `ETHERSCAN_API_KEY` set)

### Baked-in Selectors

Common function selectors compiled in via `sol!` macro — no ABI fetch needed:
- ERC20: transfer, approve, transferFrom, balanceOf, allowance, decimals, symbol, name
- Uniswap V3/V4: swap, exactInputSingle, exactOutputSingle, multicall
- Aave V3: liquidationCall, supply, borrow, repay, flashLoan
- Balancer V2: flashLoan, swap, batchSwap
- Multicall3: aggregate3, tryAggregate

Unknown selectors: query `sig.eth.samczsun.com/api/v1/signatures?function=0x...`

### ENS Resolution

ENS resolution starts on L1 mainnet. `ens` subcommand creates a separate mainnet provider. For Arbitrum addresses, requests the Arbitrum coin type explicitly.

### Benchmark Command

```bash
evmcli bench --iterations 50 --warmup 5
evmcli bench --operation balance --address 0x... --iterations 100
```

Reports: endpoint chosen, cold/warm, mean, p50, p95, p99, min, max.
Runs against both local and public RPCs if available.

### Status Command (replaces quick-check.sh)

```bash
evmcli status                     # full parallel check
evmcli status --wallet-only       # just balances
evmcli status --json              # for Claude Code
```

Configurable via `~/.config/evmcli/status.toml`:
```toml
[[wallets]]
name = "main"
address = "0x4a0aCaC60321d89E8d4d01fA09318849Cb6a586A"

[[wallets]]
name = "competitor"
address = "0x00000297dbF14d9DEb0904f88034D5C5e46C6f2f"

[contracts]
executor_v3 = "0x33643b0c0C9bc97a8aa9dAe7D894B3a63192e3D2"
```

### CLI Examples

```bash
# Reads
evmcli balance 0x4a0a...                    # ETH balance
evmcli balance 0x4a0a... --token 0xUSDC     # ERC20 balance
evmcli tx 0xHASH                            # transaction detail
evmcli receipt 0xHASH                        # receipt + decoded logs
evmcli block latest                          # latest block
evmcli call 0xCONTRACT "owner()"            # contract read
evmcli txs 0xADDR                           # tx list via Blockscout
evmcli decode 0xCALLDATA                    # decode calldata
evmcli abi 0xCONTRACT                       # fetch + cache ABI
evmcli gas                                   # current gas prices
evmcli ens resolve vitalik.eth              # ENS → address

# Writes (require PRIVATE_KEY)
evmcli transfer 0xTO 0.1                    # send ETH
evmcli transfer 0xTO 100 --token 0xUSDC     # send ERC20
evmcli approve 0xSPENDER 1000 --token 0xUSDC
evmcli send 0xCONTRACT "mint(uint256)" 100  # contract write

# Meta
evmcli status                               # parallel status check
evmcli bench                                 # benchmark
evmcli --network base balance 0xADDR        # different chain
evmcli --rpc-url http://localhost:8547 balance 0xADDR  # custom RPC
```

### Dependencies

```toml
[dependencies]
alloy = { version = "1", features = ["providers", "contract", "reqwest", "dyn-abi", "json-abi", "network"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
clap = { version = "4", features = ["derive", "env"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
comfy-table = "7"
thiserror = "2"
miette = { version = "7", features = ["fancy"] }
tracing = "0.1"
tracing-subscriber = "0.3"
directories = "5"
secrecy = { version = "0.10", features = ["serde"] }
toml = "0.8"

[dev-dependencies]
wiremock = "0.6"
```

### Testing Strategy

1. **Unit**: CLI parsing, RPC cache logic, ABI cache, output rendering snapshots
2. **Mocked RPC**: `alloy::connect_mocked_client` for exact JSON-RPC sequences
3. **Integration**: `wiremock` for Blockscout/Etherscan HTTP mocking
4. **Live smoke tests**: `EVMCLI_LIVE_TEST=1` — hit real Arbitrum with real wallet, verify outputs match `cast`
5. **Built-in `bench`**: self-testing benchmark, compare vs `cast` baseline

### Installation

```bash
cd ~/Projects/crypto-skill/evmcli
cargo build --release
# Symlink to PATH
ln -sf $(pwd)/target/release/evmcli ~/.local/bin/evmcli
```

### What It Replaces

| Before | After | Speedup |
|--------|-------|---------|
| `mcp__evm__get_balance` (3-8s) | `evmcli balance` (<500ms) | 6-16x |
| `mcp__evm__get_transaction` (3-8s) | `evmcli tx` (<500ms) | 6-16x |
| `mcp__evm__read_contract` (3-8s) | `evmcli call` (<500ms) | 6-16x |
| `ops/quick-check.sh` (0.8s) | `evmcli status` (<0.5s) | 1.6x |
| `ops/trace-tx.sh` (0.3s) | `evmcli tx --trace` (<0.2s) | 1.5x |
| `curl` Blockscout | `evmcli txs` (same speed, typed) | 1x |
| `cast 4byte-decode` | `evmcli decode` (baked sigs, faster) | 2-10x |
