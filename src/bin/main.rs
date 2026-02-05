//  Copyright 2026 PolyCrypt GmbH
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use anyhow::Result;
use ck_lightning_client::{CKLIGHTNING_LEDGER_ID, PEM_USER_ACC_PATH, PEM_NODE_ACC_PATH};
use cklightning::ic_types::SignedCandidInvoice;
use clap::Parser;
use ic_agent::Identity;
use ic_agent::{AgentError, export::Principal};
use ic_ledger_types::{AccountIdentifier, Subaccount};
use ldk_sample::{ICAgent, create_identity, str_home_from_path};
use log::{error, info, warn};
use std::io::Write;
use std::sync::OnceLock;

// Global identity path (set at startup)
static IDENTITY_PEM_PATH: OnceLock<String> = OnceLock::new();

fn get_pem_path() -> &'static str {
    IDENTITY_PEM_PATH.get().map(|s| s.as_str()).unwrap_or(PEM_USER_ACC_PATH)
}

#[derive(Parser)]
#[command(name = "ckl-cli", about = "ckLightning CLI client")]
struct Cli {
    /// Identity to use: "user" or "node"
    #[arg(short, long, default_value = "user")]
    identity: String,
}

struct MockEndpoints;
struct MockHandler;

impl MockHandler {
    fn handle_rpc(
        &mut self,
        endpoints: &mut MockEndpoints,
        client_id: u64,
        request: &str,
    ) -> Result<()> {
        info!("Mock RPC: client={}, request={:?}", client_id, request);
        // Your real handle_rpc logic here
        error!("RPC request {:?} is not supported", request);
        Err(anyhow::anyhow!("wrong_esb_msg"))
    }

    fn handle_p2p(
        &mut self,
        endpoints: &mut MockEndpoints,
        remote_id: &str,
        message: &str,
    ) -> Result<()> {
        info!("Mock P2P: remote={}, msg={:?}", remote_id, message);

        // Your real handle_p2p logic here
        match message {
            "OpenChannel" => {
                warn!("Got `open_channel` from {}, unexpected", remote_id);
            }
            "FundingLocked" | "ChannelReestablish" | "AcceptChannel" | "FundingCreated"
            | "FundingSigned" => {
                error!("Unsupported P2P request {:?} from {}", message, remote_id);
                return Err(anyhow::anyhow!("wrong_esb_msg"));
            }
            _ => {
                info!("Ignoring P2P message: {}", message);
            }
        }
        Ok(())
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Set identity based on CLI argument
    let pem_path = match cli.identity.as_str() {
        "node" => PEM_NODE_ACC_PATH,
        "user" | _ => PEM_USER_ACC_PATH,
    };
    IDENTITY_PEM_PATH.set(pem_path.to_string()).ok();

    info!("CKL CLI Started - Identity: {} ({})", cli.identity, pem_path);

    let mut handler = MockHandler;
    let mut endpoints = MockEndpoints;

    loop {
        print!("ckl> ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            break; // Ctrl+C or EOF
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0] {
            "rpc" if parts.len() > 1 => {
                let client_id = 42;
                match handler.handle_rpc(&mut endpoints, client_id, parts[1]) {
                    Ok(_) => info!("✅ RPC OK"),
                    Err(e) => error!("❌ RPC: {}", e),
                }
            }
            "p2p" if parts.len() > 1 => {
                let msg = parts[1];
                let remote_id = parts.get(2).unwrap_or(&"test-node-123");
                match handler.handle_p2p(&mut endpoints, remote_id, msg) {
                    Ok(_) => info!("✅ P2P OK"),
                    Err(e) => error!("❌ P2P: {}", e),
                }
            }
            "status" => {
                info!("🧪 Endpoints: OK | Handler: Ready");
            }
            "fetch-key" => {
                // Your ICAgent logic here
                match check_ckbtc_balance().await {
                    Ok(info) => {
                        println!("✅ {}", info);
                    }
                    Err(e) => {
                        error!("❌ FetchKey failed: {}", e);
                    }
                }

                info!("🧪 FetchKey: Root key loaded");
            }

            "ln-address" => match get_ln_address_cli().await {
                Ok(address) => {
                    println!("✅ LN Address: {}", address);
                    println!("💡 Use this address for Lightning payments!");
                }
                Err(e) => {
                    error!("❌ LN Address failed: {}", e);
                }
            },

            "ln-invoice" if parts.len() >= 3 => {
                let amount_msat: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;
                let btc_address = parts[2].to_string();

                match get_ln_invoice(amount_msat, btc_address).await {
                    Ok(invoice) => {
                        println!("✅ LN Invoice created!");
                        println!("BOLT11: {}", invoice.invoice);
                        println!("Amount: {:?} msat", invoice.amount_msat);
                        println!("Payment Hash: 0x{}", hex::encode(&invoice.payment_hash));
                        println!("Signature: {:?}", invoice.signature);
                        println!("\n💡 Copy BOLT11 above for Lightning payment!");
                    }
                    Err(e) => {
                        error!("❌ LN Invoice failed: {}", e);
                    }
                }
            }

            // =============================================================
            // Liquidity Pool Commands
            // =============================================================
            "lp-approve" if parts.len() >= 2 => {
                let amount: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;

                match lp_approve(amount).await {
                    Ok(block_idx) => {
                        println!("Approved {} satoshis for LP canister", amount);
                        println!("Block index: {}", block_idx);
                    }
                    Err(e) => {
                        error!("lp-approve failed: {}", e);
                    }
                }
            }

            "lp-deposit" if parts.len() >= 2 => {
                let amount: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;

                // Get balance before
                let balance_before = get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));
                println!("ckBTC balance before: {} satoshis", balance_before);

                match lp_deposit(amount).await {
                    Ok(resp) => {
                        if resp.success {
                            // Get balance after
                            let balance_after = get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));

                            println!("Deposited {} satoshis to LP", amount);
                            println!("New LP balance: {} satoshis", resp.new_balance);
                            println!("ckBTC balance after: {} satoshis", balance_after);
                        } else {
                            println!("Deposit failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("lp-deposit failed: {}", e);
                    }
                }
            }

            "lp-withdraw" if parts.len() >= 2 => {
                let amount: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;

                // Get balance before
                let balance_before = get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));
                println!("ckBTC balance before: {} satoshis", balance_before);

                match lp_withdraw(amount).await {
                    Ok(resp) => {
                        if resp.success {
                            // Get balance after
                            let balance_after = get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));

                            println!("Withdrew {} satoshis from LP", resp.amount_withdrawn);
                            println!("New LP balance: {} satoshis", resp.new_balance);
                            println!("ckBTC balance after: {} satoshis", balance_after);
                            if let Some(block_idx) = resp.block_index {
                                println!("Block index: {}", block_idx);
                            }
                        } else {
                            println!("Withdraw failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("lp-withdraw failed: {}", e);
                    }
                }
            }

            "lp-balance" => {
                match lp_balance().await {
                    Ok(resp) => {
                        println!("Your LP Balance:");
                        println!("  ckBTC: {} satoshis", resp.ckbtc_balance);
                        println!("  BTC:   {} satoshis", resp.btc_balance);
                    }
                    Err(e) => {
                        error!("lp-balance failed: {}", e);
                    }
                }
            }

            "lp-total" => {
                match lp_total().await {
                    Ok(resp) => {
                        println!("Total LP Balance:");
                        println!("  Total ckBTC: {} satoshis", resp.total_ckbtc);
                        println!("  Total BTC:   {} satoshis", resp.total_btc);
                        println!("  Depositors:  {}", resp.num_depositors);
                    }
                    Err(e) => {
                        error!("lp-total failed: {}", e);
                    }
                }
            }

            // =============================================================
            // BTC Liquidity Pool Commands
            // =============================================================
            "lp-btc-address" => {
                match lp_btc_address().await {
                    Ok(resp) => {
                        println!("LP BTC Deposit Address: {}", resp.address);
                        println!("");
                        println!("To deposit BTC:");
                        println!("  1. Send BTC to the address above");
                        println!("  2. Wait for 6 confirmations");
                        println!("  3. Run 'lp-btc-deposit' to claim your deposit");
                    }
                    Err(e) => {
                        error!("lp-btc-address failed: {}", e);
                    }
                }
            }

            "lp-btc-deposit" => {
                // Optional: lp-btc-deposit [txid]
                let txid: Option<Vec<u8>> = if parts.len() >= 2 {
                    // Parse txid from hex if provided
                    match hex::decode(parts[1]) {
                        Ok(bytes) => Some(bytes),
                        Err(_) => {
                            println!("Invalid txid hex format");
                            continue;
                        }
                    }
                } else {
                    None
                };

                match lp_btc_deposit(txid).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("BTC Deposit claimed!");
                            println!("  Credited: {} satoshis", resp.credited_amount);
                            println!("  New BTC balance: {} satoshis", resp.new_btc_balance);
                        } else {
                            println!("BTC Deposit failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("lp-btc-deposit failed: {}", e);
                    }
                }
            }

            "lp-btc-withdraw" if parts.len() >= 3 => {
                let amount: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;
                let destination = parts[2].to_string();

                match lp_btc_withdraw(amount, destination.clone()).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("BTC Withdrawal successful!");
                            println!("  Amount: {} satoshis", resp.amount_withdrawn);
                            println!("  Destination: {}", destination);
                            println!("  New BTC balance: {} satoshis", resp.new_btc_balance);
                            if let Some(txid) = resp.txid {
                                println!("  Transaction ID: {}", txid);
                            }
                        } else {
                            println!("BTC Withdrawal failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("lp-btc-withdraw failed: {}", e);
                    }
                }
            }

            // =============================================================
            // User BTC Operations (from depositor address)
            // =============================================================
            "btc-address" => {
                match get_depositor_btc_address().await {
                    Ok(resp) => {
                        if let Some(err) = resp.error {
                            println!("Error: {}", err);
                        } else {
                            println!("Your BTC Address: {}", resp.address);
                            println!("");
                            println!("This address is derived from your principal via threshold ECDSA.");
                            println!("Send BTC to this address to fund your account.");
                        }
                    }
                    Err(e) => {
                        error!("btc-address failed: {}", e);
                    }
                }
            }

            "btc-balance" => {
                match get_depositor_btc_balance().await {
                    Ok(resp) => {
                        if let Some(err) = resp.error {
                            println!("Error: {}", err);
                        } else {
                            println!("Your BTC Balance:");
                            println!("  Address: {}", resp.address);
                            println!("  Balance: {} satoshis", resp.balance_sat);
                        }
                    }
                    Err(e) => {
                        error!("btc-balance failed: {}", e);
                    }
                }
            }

            "btc-send" if parts.len() >= 3 => {
                let amount: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;
                let destination = parts[2].to_string();

                println!("Sending {} satoshis to {}...", amount, destination);

                match send_btc_from_depositor(amount, destination.clone()).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("BTC sent successfully!");
                            if let Some(txid) = resp.txid {
                                println!("  Transaction ID: {}", txid);
                            }
                        } else {
                            println!("Failed to send BTC: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("btc-send failed: {}", e);
                    }
                }
            }

            // =============================================================
            // Offramp Commands (ckBTC → Lightning)
            // =============================================================
            "ckbtc-balance" => {
                match get_user_ckbtc_balance().await {
                    Ok(balance) => {
                        println!("Your ckBTC Balance: {} satoshis", balance);
                    }
                    Err(e) => {
                        error!("ckbtc-balance failed: {}", e);
                    }
                }
            }

            // =============================================================
            // ICP Commands (for anti-DDoS fee)
            // =============================================================
            "icp-balance" => {
                match get_user_icp_balance().await {
                    Ok(balance) => {
                        // Balance is in e8s (1 ICP = 100_000_000 e8s)
                        // Convert BigUint to string, parse to u64 for display
                        let balance_e8s: u64 = balance.0.to_string().parse().unwrap_or(0);
                        let icp_amount = balance_e8s as f64 / 100_000_000.0;
                        println!("Your ICP Balance: {:.8} ICP ({} e8s)", icp_amount, balance);
                    }
                    Err(e) => {
                        error!("icp-balance failed: {}", e);
                    }
                }
            }

            "icp-approve" if parts.len() >= 2 => {
                let amount_icp: f64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;
                let amount_e8s = (amount_icp * 100_000_000.0) as u64;

                match icp_approve(amount_e8s).await {
                    Ok(block_idx) => {
                        println!("Approved {} ICP ({} e8s) for canister", amount_icp, amount_e8s);
                        println!("Block index: {}", block_idx);
                        println!("");
                        println!("You can now use onramp/offramp commands.");
                        println!("(20 ICP anti-DDoS fee will be collected and refunded on success)");
                    }
                    Err(e) => {
                        error!("icp-approve failed: {}", e);
                    }
                }
            }

            "offramp" if parts.len() >= 2 => {
                let invoice = parts[1].to_string();
                let fallback_addr: Option<String> = if parts.len() >= 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                };

                println!("Requesting offramp...");
                println!("Invoice: {}...", &invoice[..invoice.len().min(60)]);

                match request_offramp(invoice, fallback_addr).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("Offramp request submitted!");
                            println!("");
                            println!("  Request ID: {}", resp.request_id);
                            if let Some(amount) = resp.amount_sats {
                                println!("  Amount:     {} satoshis", amount);
                            }
                            println!("");
                            println!("The relay will pay your invoice shortly.");
                            println!("");
                            println!("To check status, run:");
                            println!("  offramp-status {}", resp.request_id);
                            println!("");
                            println!("(Use the Request ID above, NOT the invoice)");
                        } else {
                            println!("Offramp request failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("offramp failed: {}", e);
                    }
                }
            }

            "offramp-status" if parts.len() >= 2 => {
                let request_id = parts[1].to_string();

                match get_offramp_status(request_id.clone()).await {
                    Ok(resp) => {
                        println!("Offramp Status for {}:", request_id);
                        println!("  State: {:?}", resp.state);
                        println!("  Amount: {} satoshis", resp.amount_sats);
                        if let Some(err) = resp.error {
                            println!("  Error: {}", err);
                        }
                    }
                    Err(e) => {
                        error!("offramp-status failed: {}", e);
                    }
                }
            }

            // =========================================================
            // Onramp Commands (Lightning → ckBTC)
            // =========================================================

            "request-onramp" if parts.len() >= 2 => {
                let amount_sats: u64 = match parts[1].parse() {
                    Ok(amt) => amt,
                    Err(_) => {
                        println!("Invalid amount. Usage: request-onramp <amount_sats>");
                        continue;
                    }
                };

                println!("Requesting onramp invoice for {} sats...", amount_sats);
                match request_onramp_invoice(amount_sats).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("Onramp request submitted!");
                            println!("");
                            println!("  Request ID: {}", resp.request_id);
                            println!("");
                            println!("Relay will create invoice shortly. Poll with:");
                            println!("  get-invoice {}", resp.request_id);
                        } else {
                            println!("Onramp request failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("request-onramp failed: {}", e);
                    }
                }
            }

            "get-invoice" if parts.len() >= 2 => {
                let request_id = parts[1].to_string();

                match get_invoice(request_id.clone()).await {
                    Ok(resp) => {
                        println!("Invoice Status for {}:", request_id);
                        println!("  State: {:?}", resp.state);
                        if let Some(invoice) = resp.invoice {
                            println!("  Invoice: {}", invoice);
                        }
                        if let Some(err) = resp.error {
                            println!("  Error: {}", err);
                        }
                    }
                    Err(e) => {
                        error!("get-invoice failed: {}", e);
                    }
                }
            }

            // =========================================================
            // Test/Debug Commands
            // =========================================================

            "set-test-timeouts" if parts.len() >= 3 => {
                let onramp_ns: u64 = match parts[1].parse() {
                    Ok(ns) => ns,
                    Err(_) => {
                        println!("Invalid timeout. Usage: set-test-timeouts <onramp_ns> <offramp_ns>");
                        continue;
                    }
                };
                let offramp_ns: u64 = match parts[2].parse() {
                    Ok(ns) => ns,
                    Err(_) => {
                        println!("Invalid timeout. Usage: set-test-timeouts <onramp_ns> <offramp_ns>");
                        continue;
                    }
                };

                match set_test_timeouts(onramp_ns, offramp_ns).await {
                    Ok(_) => {
                        if onramp_ns == 0 && offramp_ns == 0 {
                            println!("Test timeouts reset to defaults");
                        } else {
                            println!("Test timeouts set: onramp={}ns, offramp={}ns", onramp_ns, offramp_ns);
                        }
                    }
                    Err(e) => {
                        error!("set-test-timeouts failed: {}", e);
                    }
                }
            }

            "check-expired-swaps" => {
                println!("Triggering expired swap check...");
                match check_expired_swaps().await {
                    Ok(_) => {
                        println!("Expired swap check complete");
                    }
                    Err(e) => {
                        error!("check-expired-swaps failed: {}", e);
                    }
                }
            }

            "expired-counts" => {
                match get_expired_swap_counts().await {
                    Ok((onramp, offramp)) => {
                        println!("Expired swap counts:");
                        println!("  Onramp:  {}", onramp);
                        println!("  Offramp: {}", offramp);
                    }
                    Err(e) => {
                        error!("expired-counts failed: {}", e);
                    }
                }
            }

            // =========================================================
            // Relay Registration Commands
            // =========================================================

            "register-relay" if parts.len() >= 2 => {
                let pubkey_hex = parts[1];
                let node_pubkey = match hex::decode(pubkey_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        println!("Invalid hex for node pubkey");
                        continue;
                    }
                };

                if node_pubkey.len() != 33 {
                    println!("Node pubkey must be 33 bytes (compressed secp256k1)");
                    continue;
                }

                println!("Registering relay with node pubkey: {}", pubkey_hex);
                match register_relay(node_pubkey).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("Relay registered successfully!");
                            println!("Invoice verification is now enabled.");
                        } else {
                            println!("Registration failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("register-relay failed: {}", e);
                    }
                }
            }

            "relay-info" => {
                match get_relay_info().await {
                    Ok(info) => {
                        if info.registered {
                            println!("Relay Registration:");
                            println!("  Registered: true");
                            if let Some(principal) = info.principal {
                                println!("  Principal:  {}", principal);
                            }
                            if let Some(pubkey) = info.node_pubkey {
                                println!("  Node Pubkey: {}", hex::encode(&pubkey));
                            }
                            if let Some(active) = info.is_active {
                                println!("  Active:     {}", active);
                            }
                        } else {
                            println!("No relay registered");
                        }
                    }
                    Err(e) => {
                        error!("relay-info failed: {}", e);
                    }
                }
            }

            "rate-limit" => {
                match get_rate_limit_status().await {
                    Ok(status) => {
                        println!("Rate Limit Status:");
                        println!("  Onramp:  {}/{} requests used", status.onramp_requests, status.max_onramp_per_window);
                        println!("  Offramp: {}/{} requests used", status.offramp_requests, status.max_offramp_per_window);
                        if status.window_resets_in_seconds > 0 {
                            let mins = status.window_resets_in_seconds / 60;
                            let secs = status.window_resets_in_seconds % 60;
                            println!("  Window resets in: {}m {}s", mins, secs);
                        }
                    }
                    Err(e) => {
                        error!("rate-limit failed: {}", e);
                    }
                }
            }

            "help" | "h" => {
                println!("=== ckLightning Client Commands ===");
                println!("");
                println!("User BTC (threshold ECDSA):");
                println!("  btc-address          | Get your BTC address (derived from principal)");
                println!("  btc-balance          | Check your BTC balance");
                println!("  btc-send <amt> <addr>| Send BTC from your address");
                println!("");
                println!("ICP Operations (anti-DDoS fee):");
                println!("  icp-balance          | Check your ICP balance");
                println!("  icp-approve <amount> | Approve ICP for canister (amount in ICP, e.g. 21)");
                println!("  NOTE: Onramp/offramp require 20 ICP approval (refunded on success)");
                println!("");
                println!("ckBTC Operations:");
                println!("  ckbtc-balance        | Check your ckBTC balance");
                println!("  lp-approve <amount>  | Approve canister to spend ckBTC");
                println!("");
                println!("Onramp (Lightning -> ckBTC):");
                println!("  REQUIRES: icp-approve 21");
                println!("  request-onramp <sats> | Request invoice to receive ckBTC");
                println!("  get-invoice <id>      | Get invoice for request (poll until ready)");
                println!("");
                println!("Offramp (ckBTC -> Lightning):");
                println!("  REQUIRES: icp-approve 21 && lp-approve <ckBTC amount>");
                println!("  offramp <invoice> [fallback_addr] | Exchange ckBTC for Lightning BTC");
                println!("  offramp-status <id>  | Check offramp request status");
                println!("");
                println!("ckBTC Liquidity Pool:");
                println!("  lp-deposit <amount>  | Deposit ckBTC to liquidity pool");
                println!("  lp-withdraw <amount> | Withdraw ckBTC from liquidity pool");
                println!("  lp-balance           | Show your LP balance");
                println!("  lp-total             | Show total LP balance");
                println!("");
                println!("BTC Liquidity Pool:");
                println!("  lp-btc-address       | Get shared LP BTC deposit address");
                println!("  lp-btc-deposit [txid]| Claim BTC deposit (after sending to LP address)");
                println!("  lp-btc-withdraw <amount> <address> | Withdraw BTC from LP");
                println!("");
                println!("Lightning:");
                println!("  ln-address           | Get Lightning address");
                println!("  ln-invoice <amt> <addr> | Create Lightning invoice");
                println!("");
                println!("Relay Registration (for relay operators):");
                println!("  register-relay <pubkey_hex> | Register relay with Lightning node pubkey");
                println!("  relay-info           | Show registered relay info");
                println!("");
                println!("Rate Limiting:");
                println!("  rate-limit           | Show your rate limit status");
                println!("  (Max 10 onramp + 10 offramp requests per hour)");
                println!("");
                println!("Other:");
                println!("  fetch-key            | Check ckBTC balance");
                println!("  status               | Show status");
                println!("  help                 | Show this help");
                println!("  exit/quit            | Stop");
            }
            "exit" | "quit" | "q" => break,
            _ => println!("Unknown command: {}. Type 'help' for available commands.", input),
        }
    }

    info!("👋 CKL CLI stopped");
    Ok(())
}

async fn check_ckbtc_balance() -> Result<String> {
    info!("Fetching root key with PEM: {}", get_pem_path());

    // Use your existing ICAgent from ic-lightning-relay
    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID)
        .map_err(|e| AgentError::MessageError(format!("Invalid canister ID: {}", e)))?;
    println!("ckLightning Ledger Canister ID: {:?}", can_ckl_id);

    let str_user = str_home_from_path(get_pem_path());
    let usr_user_id = create_identity(Some(&str_user));
    let usr_user_pr = usr_user_id.sender()
        .map_err(|e| AgentError::MessageError(format!("Failed to get principal from identity: {}", e)))?;
    println!("User Principal: {:?}", usr_user_pr);

    let zero_subaccount = Subaccount([0; 32]);
    let usr_acc_id = AccountIdentifier::new(&usr_user_pr, &zero_subaccount);
    println!("User Account ID: {:?}", usr_acc_id);

    // Query user's ckBTC balance
    let resp_user_balance = agent
        .icrc1_balance_of(usr_user_pr)
        .await
        .map_err(|e| AgentError::MessageError(format!("Failed to get user balance: {}", e)))?;

    Ok(format!("✅ User Balance: {}", resp_user_balance))
}

async fn get_ln_address_cli() -> Result<String, Box<dyn std::error::Error>> {
    info!("🧪 Requesting LN Address");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let client = ck_lightning_client::CkLightningClient::new(agent);
    let ln_address = client.get_ln_address().await?;

    info!("✅ Got LN address");
    Ok(ln_address)
}

// ✅ NEW: Get Lightning Invoice function
async fn get_ln_invoice(
    amount_msat: u64,
    btc_address: String,
) -> Result<SignedCandidInvoice, Box<dyn std::error::Error>> {
    info!(
        "🧪 Requesting LN Invoice: {} msat → {}",
        amount_msat, btc_address
    );

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    // Create CkLightningClient
    let client = ck_lightning_client::CkLightningClient::new(agent);

    let signed_invoice = client.query_ln_invoice(amount_msat, btc_address).await?;

    info!("✅ Got signed invoice: {} msat", amount_msat);
    Ok(signed_invoice)
}

// =============================================================================
// Liquidity Pool Helper Functions
// =============================================================================

use cklightning::ic_types::{
    LpBalanceResponse, LpDepositResponse, LpWithdrawResponse, TotalLpBalanceResponse,
    LpBtcAddressResponse, LpBtcDepositResponse, LpBtcWithdrawResponse,
    // User BTC operations
    DepositorBtcBalanceResponse, SendFromDepositorResponse,
    // Offramp types (ckBTC → Lightning)
    OfframpResponse, GetOfframpStatusResponse,
    // Onramp types (Lightning → ckBTC)
    OnrampInvoiceResponse, GetInvoiceResponse,
    // Relay registration types
    RegisterRelayResponse, GetRelayInfoResponse,
    // Rate limiting types
    RateLimitStatus,
};
use candid::Nat;

/// Approve the ckLightning canister to spend caller's ckBTC (ICRC-2)
async fn lp_approve(amount: u64) -> Result<Nat, Box<dyn std::error::Error>> {
    info!("Approving {} satoshis for LP canister", amount);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID)?;
    let block_idx = agent.tx_icrc2_approve(can_ckl_id, amount).await?;

    info!("Approval successful, block index: {}", block_idx);
    Ok(block_idx)
}

/// Deposit ckBTC to the liquidity pool
async fn lp_deposit(amount: u64) -> Result<LpDepositResponse, Box<dyn std::error::Error>> {
    info!("Depositing {} satoshis to LP", amount);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.deposit_ckbtc(amount).await?;

    info!("Deposit complete: success={}", resp.success);
    Ok(resp)
}

/// Withdraw ckBTC from the liquidity pool
async fn lp_withdraw(amount: u64) -> Result<LpWithdrawResponse, Box<dyn std::error::Error>> {
    info!("Withdrawing {} satoshis from LP", amount);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.withdraw_ckbtc(amount).await?;

    info!("Withdraw complete: success={}", resp.success);
    Ok(resp)
}

/// Get caller's LP balance
async fn lp_balance() -> Result<LpBalanceResponse, Box<dyn std::error::Error>> {
    info!("Fetching LP balance");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_my_lp_balance().await?;

    info!("LP balance fetched");
    Ok(resp)
}

/// Get total LP balance
async fn lp_total() -> Result<TotalLpBalanceResponse, Box<dyn std::error::Error>> {
    info!("Fetching total LP balance");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_total_lp_balance().await?;

    info!("Total LP balance fetched");
    Ok(resp)
}

/// Get user's on-chain ckBTC balance
async fn get_user_ckbtc_balance() -> Result<Nat, Box<dyn std::error::Error>> {
    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let str_user = str_home_from_path(get_pem_path());
    let usr_user_id = create_identity(Some(&str_user));
    let usr_user_pr = usr_user_id.sender()?;

    let balance = agent.icrc1_balance_of(usr_user_pr).await?;
    Ok(balance)
}

// =============================================================================
// BTC Liquidity Pool Helper Functions
// =============================================================================

/// Get the shared LP BTC address for deposits
async fn lp_btc_address() -> Result<LpBtcAddressResponse, Box<dyn std::error::Error>> {
    info!("Fetching LP BTC address");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_lp_btc_address().await?;

    info!("LP BTC address fetched");
    Ok(resp)
}

/// Claim a BTC deposit to the liquidity pool
///
/// After sending BTC to the shared LP address and waiting for 6 confirmations,
/// call this to credit the deposit to your LP balance.
async fn lp_btc_deposit(txid: Option<Vec<u8>>) -> Result<LpBtcDepositResponse, Box<dyn std::error::Error>> {
    info!("Claiming BTC deposit");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.deposit_btc(txid, 0).await?;

    info!("BTC deposit claim complete: success={}", resp.success);
    Ok(resp)
}

/// Withdraw BTC from the liquidity pool
async fn lp_btc_withdraw(
    amount: u64,
    destination: String,
) -> Result<LpBtcWithdrawResponse, Box<dyn std::error::Error>> {
    info!("Withdrawing {} satoshis BTC to {}", amount, destination);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.withdraw_btc(amount, destination).await?;

    info!("BTC withdraw complete: success={}", resp.success);
    Ok(resp)
}

// =============================================================================
// User BTC Operations Helper Functions (from depositor address)
// =============================================================================

/// Get the caller's BTC address (derived from principal via threshold ECDSA)
async fn get_depositor_btc_address() -> Result<DepositorBtcBalanceResponse, Box<dyn std::error::Error>> {
    info!("Fetching depositor BTC address");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_depositor_btc_balance().await?;

    info!("Depositor BTC address fetched");
    Ok(resp)
}

/// Get the caller's BTC balance at their depositor address
async fn get_depositor_btc_balance() -> Result<DepositorBtcBalanceResponse, Box<dyn std::error::Error>> {
    info!("Fetching depositor BTC balance");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_depositor_btc_balance().await?;

    info!("Depositor BTC balance fetched: {} sats", resp.balance_sat);
    Ok(resp)
}

/// Send BTC from the caller's depositor address to a destination
async fn send_btc_from_depositor(
    amount: u64,
    destination: String,
) -> Result<SendFromDepositorResponse, Box<dyn std::error::Error>> {
    info!("Sending {} satoshis BTC to {}", amount, destination);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.send_btc_from_depositor_address(amount, destination).await?;

    info!("BTC send complete: success={}", resp.success);
    Ok(resp)
}

// =============================================================================
// Offramp Helper Functions (ckBTC → Lightning)
// =============================================================================

/// Request an offramp (ckBTC → Lightning)
///
/// User must first call lp-approve to allow the canister to take custody of ckBTC.
/// The canister takes custody of the ckBTC and the relay pays the user's invoice.
async fn request_offramp(
    invoice: String,
    fallback_btc_address: Option<String>,
) -> Result<OfframpResponse, Box<dyn std::error::Error>> {
    info!("Requesting offramp with invoice: {}...", &invoice[..invoice.len().min(40)]);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.request_offramp(invoice, fallback_btc_address).await?;

    info!("Offramp request complete: success={}", resp.success);
    Ok(resp)
}

/// Get the status of an offramp request
async fn get_offramp_status(
    request_id: String,
) -> Result<GetOfframpStatusResponse, Box<dyn std::error::Error>> {
    info!("Fetching offramp status for: {}", request_id);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_offramp_status(request_id).await?;

    info!("Offramp status fetched");
    Ok(resp)
}

// =============================================================================
// ICP Operations Helper Functions (for anti-DDoS fee)
// =============================================================================


/// Get user's ICP balance
async fn get_user_icp_balance() -> Result<Nat, Box<dyn std::error::Error>> {
    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let str_user = str_home_from_path(get_pem_path());
    let usr_user_id = create_identity(Some(&str_user));
    let usr_user_pr = usr_user_id.sender()?;

    let balance = agent.icp_balance_of(usr_user_pr).await?;
    Ok(balance)
}

/// Approve the ckLightning canister to spend caller's ICP (ICRC-2)
async fn icp_approve(amount_e8s: u64) -> Result<Nat, Box<dyn std::error::Error>> {
    info!("Approving {} e8s ICP for canister", amount_e8s);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID)?;
    let block_idx = agent.tx_icp_icrc2_approve(can_ckl_id, amount_e8s).await?;

    info!("ICP approval successful, block index: {}", block_idx);
    Ok(block_idx)
}

// =============================================================================
// Onramp Helper Functions (Lightning → ckBTC)
// =============================================================================

/// Request an onramp invoice (Lightning → ckBTC)
///
/// User must first call icp-approve to allow the canister to collect the 20 ICP anti-DDoS fee.
/// The fee is refunded on successful completion.
async fn request_onramp_invoice(
    amount_sats: u64,
) -> Result<OnrampInvoiceResponse, Box<dyn std::error::Error>> {
    info!("Requesting onramp invoice for {} sats", amount_sats);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    // Get the user's principal (recipient of ckBTC)
    let str_user = str_home_from_path(get_pem_path());
    let usr_user_id = create_identity(Some(&str_user));
    let recipient = usr_user_id.sender()?;

    let resp = agent.request_onramp_invoice(recipient, amount_sats).await?;

    info!("Onramp invoice request complete: success={}", resp.success);
    Ok(resp)
}

/// Get the status/invoice for an onramp request
async fn get_invoice(
    request_id: String,
) -> Result<GetInvoiceResponse, Box<dyn std::error::Error>> {
    info!("Fetching invoice for request: {}", request_id);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_invoice(request_id).await?;

    info!("Invoice status fetched");
    Ok(resp)
}

// =============================================================================
// Test/Debug Helper Functions
// =============================================================================

/// Set test timeouts for swap expiry (for E2E testing)
/// Pass 0,0 to reset to defaults
async fn set_test_timeouts(
    onramp_ns: u64,
    offramp_ns: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting test timeouts: onramp={}ns, offramp={}ns", onramp_ns, offramp_ns);

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    agent.set_test_timeouts(onramp_ns, offramp_ns).await?;

    info!("Test timeouts set");
    Ok(())
}

/// Manually trigger expired swap check
async fn check_expired_swaps() -> Result<(), Box<dyn std::error::Error>> {
    info!("Triggering expired swap check");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    agent.check_expired_swaps().await?;

    info!("Expired swap check complete");
    Ok(())
}

/// Get expired swap counts
async fn get_expired_swap_counts() -> Result<(u64, u64), Box<dyn std::error::Error>> {
    info!("Fetching expired swap counts");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let (onramp, offramp) = agent.get_expired_swap_counts().await?;

    info!("Expired counts: onramp={}, offramp={}", onramp, offramp);
    Ok((onramp, offramp))
}

// =============================================================================
// Relay Registration Helper Functions
// =============================================================================

/// Register a relay with its Lightning node pubkey
///
/// The relay must call this before submitting invoices for onramp requests.
/// This enables invoice verification to prevent invoice substitution attacks.
async fn register_relay(node_pubkey: Vec<u8>) -> Result<RegisterRelayResponse, Box<dyn std::error::Error>> {
    info!("Registering relay with node pubkey: {}", hex::encode(&node_pubkey));

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.register_relay(node_pubkey).await?;

    info!("Relay registration complete: success={}", resp.success);
    Ok(resp)
}

/// Get information about the registered relay
async fn get_relay_info() -> Result<GetRelayInfoResponse, Box<dyn std::error::Error>> {
    info!("Fetching relay info");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_relay_info().await?;

    info!("Relay info fetched: registered={}", resp.registered);
    Ok(resp)
}

/// Get the caller's rate limit status
async fn get_rate_limit_status() -> Result<RateLimitStatus, Box<dyn std::error::Error>> {
    info!("Fetching rate limit status");

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
    agent.fetch_root_key().await?;

    let resp = agent.get_rate_limit_status().await?;

    info!("Rate limit status fetched");
    Ok(resp)
}
