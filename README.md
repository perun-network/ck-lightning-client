# ckLightning Client

CLI tool for interacting with the ckLightning canister. Supports LP operations, swap requests, admin configuration, and balance queries.

## Related Components

| Component | Repo | Role |
|-----------|------|------|
| **ckLightning-canister** | [perun-network/ckLightning-canister](https://github.com/perun-network/ckLightning-canister) | IC canister — LP management, swap state, channel signing |
| **ic-lightning-relay** | [perun-network/ic-lightning-relay](https://github.com/perun-network/ic-lightning-relay) | LDK Lightning node — swap execution, invoice creation, channel management |

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed component design.

## Build

```bash
cargo build --release --bin main
```

## Usage

```bash
./target/release/main [--identity <user|node|default|dfx_identity_name>]
```

### Commands

**Swaps:**
- `request-onramp <amount_sats>` — Request onramp (Lightning → ckBTC)
- `get-invoice <request_id>` — Poll invoice status
- `offramp <invoice> [fallback_addr]` — Request offramp (ckBTC → Lightning)
- `offramp-status <request_id>` — Check offramp status

**Liquidity Pool (ckBTC):**
- `lp-approve <amount>` — Approve canister to spend ckBTC
- `lp-deposit <amount>` — Deposit ckBTC to LP
- `lp-withdraw <amount>` — Withdraw ckBTC from LP
- `lp-balance` / `lp-total` — Show LP balances

**Liquidity Pool (BTC):**
- `lp-btc-address` — Get shared LP BTC deposit address
- `lp-btc-deposit [txid]` — Claim BTC deposit
- `lp-btc-withdraw <amount> <address>` — Withdraw BTC from LP

**User BTC (threshold ECDSA):**
- `btc-address` — Get your BTC address (derived from principal)
- `btc-balance` — Check BTC balance at your depositor address
- `btc-send <amount> <destination>` — Send BTC from your personal address

**Lightning:**
- `ln-address` — Get Lightning address
- `ln-invoice <amount_msat> <btc_address>` — Create Lightning invoice

**Balances:**
- `icp-balance` / `ckbtc-balance` — Check ICP/ckBTC balances
- `icp-approve <amount>` — Approve ICP for anti-DDoS fee
- `fetch-key` — Check ckBTC balance (alias)

**Admin (requires admin/controller identity):**
- `set-admin <principal>` — Set admin principal
- `update-config [--amp N] [--fee N] [--protocol-fee N] [--slippage N] [--imbalance-fee N] [--rebate N] [--max-swap-pct N]` — Update StableSwap parameters
- `withdraw-fees <principal>` — Withdraw protocol fees
- `register-relay <pubkey_hex> [webhook_url] [webhook_token]` — Register relay node
- `relay-info` — Show registered relay information

**Diagnostics:**
- `whoami` — Show current principal
- `rate-limit` — Show rate limit status
- `pending-onramps` — List pending onramp requests
- `pending-offramps` — List pending offramp requests

**Testing:**
- `set-test-timeouts <onramp_ns> <offramp_ns>` — Set swap timeouts (testing)
- `check-expired-swaps` — Manually trigger expired swap check
- `expired-counts` — Show count of expired swaps

**General:**
- `help` / `h` — Show help
- `exit` / `quit` / `q` — Exit

## Prerequisites

- Rust toolchain (stable)
- `dfx` with local IC environment and PEM identities
- Canister deployed via `setup_all.sh`

## Copyright

Copyright 2026 PolyCrypt GmbH. Use of the source code is governed by the Apache 2.0 license that can be found in the [LICENSE file](LICENSE).
