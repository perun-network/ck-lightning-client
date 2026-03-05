# ckLightning Client

CLI tool for interacting with the ckLightning canister. Supports LP operations, swap requests, admin configuration, and balance queries.

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

**Balances:**
- `icp-balance` / `ckbtc-balance` / `btc-balance` — Check balances
- `icp-approve <amount>` — Approve ICP for anti-DDoS fee

**Admin (requires admin/controller identity):**
- `set-admin <principal>` — Set admin principal
- `update-config [--amp N] [--fee N] ...` — Update StableSwap parameters
- `withdraw-fees <principal>` — Withdraw protocol fees
- `register-relay <pubkey_hex>` — Register relay node

**Other:**
- `whoami` — Show current principal
- `status` — Show system status
- `rate-limit` — Show rate limit status

## Prerequisites

- Rust toolchain (stable)
- `dfx` with local IC environment and PEM identities
- Canister deployed via `setup_all.sh`

## Copyright

Copyright 2026 PolyCrypt GmbH. Use of the source code is governed by the Apache 2.0 license that can be found in the [LICENSE file](LICENSE).
