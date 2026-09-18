# Using ckLightning

How to swap between Lightning BTC and ckBTC, and how to provide liquidity, with the `ckLightning-client`
REPL. Written 2026-09-17 against the client on `staging-april-deployment`.

Everything below assumes the staging setup: IC mainnet, **ckTESTBTC** and Bitcoin **testnet4**. Those
tokens have no monetary value. Only the ICP anti-DDoS fee is real money.

---

## 1. Before you start

You need a dfx identity whose **plaintext** PEM the client can read, the canister reachable over
`IC_URL`, and a relay that is registered and running. Check the last one first — swaps do nothing without
it:

```bash
IC_URL=https://ic0.app ./target/release/main -i staging-user
ckl> relay-info
```

`Registered: true` with a node pubkey means the bridge is staffed. `No relay registered` means nothing
will be paid out, no matter what you submit.

```
ckl> whoami          # your principal
ckl> ckbtc-balance   # ckBTC you hold
ckl> icp-balance     # ICP for the anti-DDoS fee
```

## 2. Amounts, fees and approvals

- Amounts are **satoshis**, except `icp-approve`, which takes **ICP as a decimal** (`icp-approve 0.0015`),
  and `ln-invoice`, which takes millisatoshis.
- Every swap request collects an **ICP anti-DDoS fee**, refunded when the swap succeeds and kept when it
  fails or expires. The operator sets it: the code default is 1 ICP, the April staging deployment used
  0.001 ICP. Ask the operator, or read it yourself:
  `dfx canister call <canister id> get_icp_ddos_fee '()' --network ic --query`. Approve the fee plus the
  ICP ledger fee of 0.0001 and some slack — `icp-approve 0.0015` against the staging value,
  `icp-approve 2` against the default.
- Paying ckBTC to the canister (offramp, LP deposit) needs an ICRC-2 approval that covers the amount plus
  the ledger transfer fee (10 sats): for a 10 000-sat offramp use `lp-approve 11000`.
- Swap pricing follows a StableSwap curve: a small fee, higher when your swap pushes the pool further out
  of balance, plus a slippage limit that rejects extreme swaps. Preview a swap before committing — from
  the relay REPL, `swap-quote btc2ckbtc 20000`.
- You may make 10 onramp and 10 offramp requests per hour; `rate-limit` shows your counters.

## 3. Onramp: Lightning BTC → ckBTC

```
ckl> icp-approve 0.0015
ckl> request-onramp 20000
Onramp request submitted!

  Request ID: <id>

Relay will create invoice shortly. Poll with:
  get-invoice <id>
```

The canister asks the relay for an invoice. Poll until the state is `Ready`:

```
ckl> get-invoice <id>
Invoice Status for <id>:
  State: Ready
  Invoice: lnbc...
```

Pay that BOLT11 invoice from any Lightning wallet. When the relay has claimed the payment it calls the
canister, which prices the swap, pays you the ckBTC and refunds the ICP fee. Confirm with
`ckbtc-balance`; you receive slightly less than you sent because of the swap fee.

The invoice expires after 25 minutes and the request after 30. If you pay too late the ICP fee is kept
and the payment may not turn into ckBTC — do not pay an expired invoice.

## 4. Offramp: ckBTC → Lightning BTC

Create an invoice in your own Lightning wallet for the amount you want to receive, then:

```
ckl> icp-approve 0.0015
ckl> lp-approve 11000            # amount + ledger fee + margin
ckl> offramp <bolt11>
Offramp request submitted!

  Request ID: <id>
  Amount:     10000 satoshis

ckl> offramp-status <id>
```

The canister takes the ckBTC into custody immediately, then the relay pays your invoice. `Completed`
means paid and the ICP fee refunded. If the payment fails, the ckBTC is returned and the ICP fee is kept.
If nothing happens within 10 minutes the request expires and the ckBTC is refunded.

Use the **Request ID** for `offramp-status`, not the invoice.

## 5. Providing liquidity

Swaps are paid out of the pool, so both sides need funding: ckBTC for onramps, BTC for channels and
offramp settlement. Fees earned by the pool are credited to depositors in proportion to their share.

**ckBTC side**

```
ckl> lp-approve 151000     # amount + ledger fee
ckl> lp-deposit 150000
ckl> lp-balance            # your share
ckl> lp-total              # pool total and number of depositors
ckl> lp-withdraw 50000
```

**BTC side**

```
ckl> lp-btc-address        # your personal deposit address
# send testnet4 BTC to it, then wait for confirmations
ckl> lp-btc-deposit
```

The address belongs to you alone: only your own deposits are credited to you. Two waits apply before the
claim works — the confirmations the canister requires (1 on the staging build, 6 on the default branch;
the client's message always says 6) and the IC Bitcoin canister's own lag, which was about 20 minutes on
testnet4. If `lp-btc-deposit` reports no new deposits, wait and retry.

```
ckl> lp-btc-withdraw 50000 <btc-address>
```

Withdrawals cover only BTC that is not locked in a channel.

## 6. Your own BTC address

`btc-address` and `btc-balance` show a personal address derived from your principal, and
`btc-send <sats> <address>` spends from it. This is separate from the LP pool: BTC there is yours and is
not used for swaps.

Do not use the legacy `set-btc-address` path or the `get_p2*_address` endpoints. They derive a fixed
address that is not tied to your principal, so several users can end up sharing one address.

## 7. When something goes wrong

| Symptom | Cause | What to do |
|---|---|---|
| `No relay registered` | No relay is registered with the canister | Nothing will settle; contact the operator |
| Invoice stays `Pending` | Relay is down or not polling | Check `relay-info`; wait one polling interval (30 s) |
| `Failed to collect ICP anti-DDoS fee: … Did you approve N ICP?` | Missing or too small ICP approval | `icp-approve` above the current fee plus 0.0001, then retry |
| `InsufficientAllowance` on offramp or LP deposit | ckBTC approval below amount + fee | `lp-approve <amount + 10 + margin>` |
| Offramp ends as `Failed`/`Refunded` | Lightning payment failed (no route, expired invoice) | ckBTC is back; the ICP fee is not refunded. Retry with a fresh invoice and longer expiry |
| Swap rejected for slippage or size | The pool is too imbalanced or the swap is too large | Swap a smaller amount or wait for the pool to rebalance |
| `Rate limited: 10 … requests per hour exceeded. Try again in N seconds.` | More than 10 requests in an hour | Wait; `rate-limit` shows your counters |
| `BTC Deposit failed: … No new deposits found with N confirmations` | Confirmations or the IC Bitcoin lag | Wait and retry `lp-btc-deposit` |
| Client eats a CPU core | stdin was piped into the REPL | Run it interactively or through `expect` |

A completed swap can be checked without trusting anyone: the preimage must hash to the payment hash.

```bash
python3 -c "import hashlib; print(hashlib.sha256(bytes.fromhex('<preimage>')).hexdigest())"
```

## 8. Operator commands

`register-relay`, `set-admin`, `update-config`, `withdraw-fees`, `pending-onramps`, `pending-offramps`,
`set-test-timeouts`, `check-expired-swaps` and `expired-counts` are for whoever runs the bridge and need
the admin, controller or relay identity. See `OPERATIONS.md` in the canister and relay repositories.
