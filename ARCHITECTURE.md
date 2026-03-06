# Architecture

## Overview

The ckLightning client is a Rust CLI tool providing a REPL interface for interacting with the ckLightning canister. It is stateless — all state lives on the canister. The client handles identity management, argument parsing, canister calls via `ic-agent`, and result formatting.

## Structure

```
src/
├── bin/
│   ├── main.rs              # Entry point, REPL loop, command dispatch
│   └── commands/
│       ├── mod.rs            # Module declarations
│       ├── admin.rs          # Admin, relay, config, diagnostics
│       ├── lp.rs             # LP deposit/withdraw/balance (ckBTC + BTC)
│       ├── swap.rs           # Onramp/offramp requests, ICP/ckBTC balance
│       └── btc.rs            # User BTC operations (depositor address)
├── lib.rs                    # Public API exports
├── client.rs                 # CkLightningClient wrapper around ICAgent
├── ic_params.rs              # Constants (canister IDs, PEM paths, fees)
├── invoice.rs                # CandidInvoice / SignedCandidInvoice types
└── candid_invoice.rs         # BOLT11 ↔ Candid invoice conversion
```

## REPL Loop (main.rs)

```
Start → Parse --identity flag → Store PEM path in OnceLock
     → Loop:
         Print "ckl> " → Read line → Split tokens
         → Match command → Parse args → Create ICAgent → Call canister
         → Print result → Loop
```

- `--identity` flag selects PEM file: `user` (default), `node`, `default`, or any dfx identity name
- Identity persists for the entire session via `OnceLock<String>`
- Each command creates a fresh `ICAgent` from the stored PEM path
- Errors are logged and printed but never crash the REPL

## Identity Management

The client uses dfx PEM identity files for authentication:

| Identity | PEM Path | Purpose |
|----------|----------|---------|
| `user` | `~/.config/dfx/identity/user/identity.pem` | End-user operations (default) |
| `node` | `~/.config/dfx/identity/node/identity.pem` | Node operator operations |
| Custom | `~/.config/dfx/identity/<name>/identity.pem` | Any dfx identity |

The principal is derived from the PEM keypair — no explicit authentication step needed.

## Canister Communication

Every command follows the same pattern:

```rust
let agent = ICAgent::new_from_pem_file(Some(pem_path))?;
agent.fetch_root_key().await?;  // required for local dfx
let response = agent.canister_method(args).await?;
```

The `ICAgent` is imported from the `ic-lightning-relay` crate. Canister IDs are hardcoded in `ic_params.rs`:

| Canister | ID | Purpose |
|----------|----|---------|
| `cklightning` | `vizcg-th777-77774-qaaea-cai` | Main ckLightning canister |
| `btcledger` | `u6s2n-gx777-77774-qaaba-cai` | ckBTC ledger |
| `ledger` | `ufxgi-4p777-77774-qaadq-cai` | ICP ledger |
| `minter` | `uzt4z-lp777-77774-qaabq-cai` | Bitcoin minter |

## Command Categories

### Swap Commands (swap.rs)

| Command | Canister Call | Flow |
|---------|--------------|------|
| `request-onramp <sats>` | `request_onramp_invoice` | User starts onramp, gets request_id |
| `get-invoice <id>` | `get_invoice_by_request` | Poll until invoice is Ready |
| `offramp <invoice>` | `request_offramp` | Submit BOLT11 invoice for ckBTC→Lightning |
| `offramp-status <id>` | `get_offramp_status` | Poll offramp state |
| `icp-approve <amount>` | ICRC-2 approve on ICP ledger | Approve ICP for anti-DDoS fee |
| `icp-balance` | ICRC-1 balance on ICP ledger | Check ICP balance |
| `ckbtc-balance` | ICRC-1 balance on ckBTC ledger | Check ckBTC balance |

### LP Commands (lp.rs)

| Command | Canister Call |
|---------|--------------|
| `lp-approve <sats>` | ICRC-2 approve on ckBTC ledger |
| `lp-deposit <sats>` | `deposit_ckbtc` |
| `lp-withdraw <sats>` | `withdraw_ckbtc` |
| `lp-balance` | `get_my_lp_balance` |
| `lp-total` | `get_total_lp_balance` |
| `lp-btc-address` | `get_lp_btc_address` |
| `lp-btc-deposit [txid]` | `deposit_btc` |
| `lp-btc-withdraw <sats> <addr>` | `withdraw_btc` |

### Admin Commands (admin.rs)

| Command | Canister Call | Access |
|---------|--------------|--------|
| `set-admin <principal>` | `set_admin` | Controller only |
| `register-relay <pubkey>` | `register_relay` | Admin only |
| `update-config [--amp N] ...` | `update_stableswap_config` | Admin only |
| `withdraw-fees <principal>` | `withdraw_protocol_fees` | Admin only |
| `relay-info` | `get_relay_info` | Any |
| `rate-limit` | `get_rate_limit_status` | Any |
| `whoami` | Local principal derivation | Any |
| `fetch-key` | ICRC-1 balance query | Any |
| `ln-address` | `get_ln_address` | Any |
| `ln-invoice <msat> <addr>` | `query_ln_invoice` | Any |

### BTC Commands (btc.rs)

| Command | Canister Call |
|---------|--------------|
| `btc-address` | `get_depositor_btc_address` |
| `btc-balance` | `get_depositor_btc_balance` |
| `btc-send <sats> <addr>` | `send_btc_from_depositor` |

### Diagnostics

| Command | Purpose |
|---------|---------|
| `pending-onramps` | List pending onramp requests |
| `pending-offramps` | List pending offramp requests |
| `set-test-timeouts <on> <off>` | Override swap timeouts (testing) |
| `check-expired-swaps` | Trigger expired swap cleanup |
| `expired-counts` | Show expired swap counts |

## Invoice Conversion (candid_invoice.rs)

Bridges between BOLT11 Lightning invoices and IC Candid types:

- `bolt11_to_candid(invoice, signature, channel_id)` → `SignedCandidInvoice`
- `candid_to_bolt11(candid_invoice)` → `Bolt11Invoice`

Validates that metadata (payment_hash, amount, timestamp) is consistent between formats.

## Error Handling

- All command functions return `Result<T, Box<dyn std::error::Error>>`
- Errors propagated with `?`, logged via `log::error!()`, printed to stdout
- REPL continues on error — no panics
- Canister responses include `success: bool` and `error: Option<String>` fields
