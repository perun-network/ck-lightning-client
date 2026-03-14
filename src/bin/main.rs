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

mod commands;

use anyhow::Result;
use candid::Nat;
use ck_lightning_client::{PEM_USER_ACC_PATH, PEM_NODE_ACC_PATH};
use clap::Parser;
use log::{error, info};
use std::io::Write;
use std::sync::OnceLock;

/// 1 ICP = 100,000,000 e8s (the smallest ICP unit).
const E8S_PER_ICP: u64 = 100_000_000;

// Global identity path (set at startup)
static IDENTITY_PEM_PATH: OnceLock<String> = OnceLock::new();

fn get_pem_path() -> &'static str {
    IDENTITY_PEM_PATH.get().map(|s| s.as_str()).unwrap_or(PEM_USER_ACC_PATH)
}

#[derive(Parser)]
#[command(name = "ckl-cli", about = "ckLightning CLI client")]
struct Cli {
    /// Identity to use: "user", "node", "default", or any dfx identity name
    #[arg(short, long, default_value = "user")]
    identity: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Set identity (user, node, default, or custom dfx identity name)
    let pem_path = match cli.identity.as_str() {
        "node" => PEM_NODE_ACC_PATH.to_string(),
        "user" => PEM_USER_ACC_PATH.to_string(),
        name => {
            // Reject path traversal characters in identity names
            if name.contains('/') || name.contains('\\') || name.contains("..") {
                eprintln!("Invalid identity name: must not contain path separators or '..'");
                std::process::exit(1);
            }
            format!(".config/dfx/identity/{}/identity.pem", name)
        }
    };
    IDENTITY_PEM_PATH.set(pem_path.clone()).ok();

    info!("CKL CLI Started - Identity: {}", cli.identity);

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
            "fetch-key" => {
                match commands::admin::check_ckbtc_balance().await {
                    Ok(info) => {
                        println!("{}", info);
                    }
                    Err(e) => {
                        error!("FetchKey failed: {}", e);
                    }
                }

                info!("FetchKey: Root key loaded");
            }

            "ln-address" => match commands::admin::get_ln_address_cli().await {
                Ok(address) => {
                    println!("LN Address: {}", address);
                }
                Err(e) => {
                    error!("LN Address failed: {}", e);
                }
            },

            "ln-invoice" if parts.len() >= 3 => {
                let amount_msat: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid amount"))?;
                let btc_address = parts[2].to_string();

                match commands::admin::get_ln_invoice(amount_msat, btc_address).await {
                    Ok(invoice) => {
                        println!("LN Invoice created!");
                        println!("BOLT11: {}", invoice.invoice);
                        println!("Amount: {:?} msat", invoice.amount_msat);
                        println!("\nCopy BOLT11 above for Lightning payment!");
                    }
                    Err(e) => {
                        error!("LN Invoice failed: {}", e);
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

                match commands::lp::lp_approve(amount).await {
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
                let balance_before = commands::swap::get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));
                println!("ckBTC balance before: {} satoshis", balance_before);

                match commands::lp::lp_deposit(amount).await {
                    Ok(resp) => {
                        if resp.success {
                            // Get balance after
                            let balance_after = commands::swap::get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));

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
                let balance_before = commands::swap::get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));
                println!("ckBTC balance before: {} satoshis", balance_before);

                match commands::lp::lp_withdraw(amount).await {
                    Ok(resp) => {
                        if resp.success {
                            // Get balance after
                            let balance_after = commands::swap::get_user_ckbtc_balance().await.unwrap_or(Nat::from(0u64));

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
                match commands::lp::lp_balance().await {
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
                match commands::lp::lp_total().await {
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
                match commands::lp::lp_btc_address().await {
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

                match commands::lp::lp_btc_deposit(txid).await {
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

                match commands::lp::lp_btc_withdraw(amount, destination.clone()).await {
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
                match commands::btc::get_depositor_btc_address().await {
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
                match commands::btc::get_depositor_btc_balance().await {
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

                match commands::btc::send_btc_from_depositor(amount, destination.clone()).await {
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
            // Offramp Commands (ckBTC -> Lightning)
            // =============================================================
            "ckbtc-balance" => {
                match commands::swap::get_user_ckbtc_balance().await {
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
                match commands::swap::get_user_icp_balance().await {
                    Ok(balance) => {
                        // Balance is in e8s (1 ICP = 100_000_000 e8s)
                        let balance_e8s: u64 = match balance.0.to_string().parse() {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Error: balance too large to display as u64");
                            continue;
                        }
                    };
                        let whole = balance_e8s / E8S_PER_ICP;
                        let frac = balance_e8s % E8S_PER_ICP;
                        println!("Your ICP Balance: {}.{:08} ICP ({} e8s)", whole, frac, balance);
                    }
                    Err(e) => {
                        error!("icp-balance failed: {}", e);
                    }
                }
            }

            "icp-approve" if parts.len() >= 2 => {
                let amount_str = parts[1];
                // Parse as e8s: support both whole (2) and decimal (0.5) ICP
                let amount_e8s: u64 = if let Some(dot_pos) = amount_str.find('.') {
                    let whole: u64 = match amount_str[..dot_pos].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Invalid ICP amount: {}", amount_str);
                            continue;
                        }
                    };
                    let frac_str = &amount_str[dot_pos + 1..];
                    let frac_padded = format!("{:0<8}", frac_str);
                    let frac: u64 = match frac_padded[..8].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Invalid ICP fraction: {}", amount_str);
                            continue;
                        }
                    };
                    whole * E8S_PER_ICP + frac
                } else {
                    let whole: u64 = amount_str.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid amount (e.g. 2 or 0.5)"))?;
                    whole * E8S_PER_ICP
                };

                match commands::swap::icp_approve(amount_e8s).await {
                    Ok(block_idx) => {
                        println!("Approved {} ICP ({} e8s) for canister", amount_str, amount_e8s);
                        println!("Block index: {}", block_idx);
                        println!("");
                        println!("You can now use onramp/offramp commands.");
                        println!("(1 ICP anti-DDoS fee will be collected and refunded on success)");
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

                match commands::swap::request_offramp(invoice, fallback_addr).await {
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

                match commands::swap::get_offramp_status(request_id.clone()).await {
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
            // Onramp Commands (Lightning -> ckBTC)
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
                match commands::swap::request_onramp_invoice(amount_sats).await {
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

                match commands::swap::get_invoice(request_id.clone()).await {
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

                match commands::admin::set_test_timeouts(onramp_ns, offramp_ns).await {
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
                match commands::admin::check_expired_swaps().await {
                    Ok(_) => {
                        println!("Expired swap check complete");
                    }
                    Err(e) => {
                        error!("check-expired-swaps failed: {}", e);
                    }
                }
            }

            "expired-counts" => {
                match commands::admin::get_expired_swap_counts().await {
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

                // Optional: register-relay <pubkey> [webhook_url] [webhook_token]
                let webhook_url = parts.get(2).map(|s| s.to_string());
                let webhook_token = parts.get(3).map(|s| s.to_string());

                println!("Registering relay with node pubkey: {}", pubkey_hex);
                if let Some(ref url) = webhook_url {
                    println!("  Webhook URL: {}", url);
                }
                match commands::admin::register_relay(node_pubkey, webhook_url, webhook_token).await {
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
                match commands::admin::get_relay_info().await {
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
                            match &info.relay_http_url {
                                Some(url) => println!("  Webhook URL: {}", url),
                                None => println!("  Webhook URL: not configured"),
                            }
                            match info.has_auth_token {
                                Some(true) => println!("  Auth Token:  configured"),
                                _ => println!("  Auth Token:  not configured"),
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
                match commands::admin::get_rate_limit_status().await {
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

            // =============================================================
            // Admin / Identity Commands
            // =============================================================

            "whoami" => {
                match commands::admin::whoami().await {
                    Ok(principal) => {
                        println!("{}", principal);
                    }
                    Err(e) => {
                        error!("whoami failed: {}", e);
                    }
                }
            }

            "set-admin" if parts.len() >= 2 => {
                let principal_text = parts[1];
                match commands::admin::set_admin(principal_text.to_string()).await {
                    Ok(_) => {
                        println!("Admin set to: {}", principal_text);
                    }
                    Err(e) => {
                        error!("set-admin failed: {}", e);
                    }
                }
            }

            "withdraw-fees" if parts.len() >= 2 => {
                let recipient_text = parts[1];
                match commands::admin::withdraw_protocol_fees(recipient_text.to_string()).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("Protocol fees withdrawn!");
                            println!("  BTC:   {} satoshis", resp.btc_amount);
                            println!("  ckBTC: {} satoshis", resp.ckbtc_amount);
                            if let Some(block_idx) = resp.ckbtc_block_index {
                                println!("  Block index: {}", block_idx);
                            }
                        } else {
                            println!("Withdraw failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("withdraw-fees failed: {}", e);
                    }
                }
            }

            "update-config" => {
                // Parse optional named args: --amp N --fee N --protocol-fee N --slippage N --imbalance-fee N --rebate N --max-swap-pct N
                let mut amplification: Option<u64> = None;
                let mut fee_bps: Option<u64> = None;
                let mut protocol_fee_share_bps: Option<u64> = None;
                let mut max_slippage_bps: Option<u64> = None;
                let mut imbalance_fee_bps: Option<u64> = None;
                let mut rebate_bps: Option<u64> = None;
                let mut max_swap_pct_bps: Option<u64> = None;

                let mut i = 1;
                while i < parts.len() {
                    match parts[i] {
                        "--amp" if i + 1 < parts.len() => {
                            amplification = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--fee" if i + 1 < parts.len() => {
                            fee_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--protocol-fee" if i + 1 < parts.len() => {
                            protocol_fee_share_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--slippage" if i + 1 < parts.len() => {
                            max_slippage_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--imbalance-fee" if i + 1 < parts.len() => {
                            imbalance_fee_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--rebate" if i + 1 < parts.len() => {
                            rebate_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        "--max-swap-pct" if i + 1 < parts.len() => {
                            max_swap_pct_bps = parts[i + 1].parse().ok();
                            i += 2;
                        }
                        _ => {
                            println!("Unknown arg: {}. Usage: update-config [--amp N] [--fee N] [--protocol-fee N] [--slippage N] [--imbalance-fee N] [--rebate N] [--max-swap-pct N]", parts[i]);
                            i = parts.len(); // break
                        }
                    }
                }

                match commands::admin::update_stableswap_config(
                    amplification,
                    fee_bps,
                    protocol_fee_share_bps,
                    max_slippage_bps,
                    imbalance_fee_bps,
                    rebate_bps,
                    max_swap_pct_bps,
                ).await {
                    Ok(resp) => {
                        if resp.success {
                            println!("StableSwap config updated!");
                            println!("  Amplification:      {}", resp.config.amplification);
                            println!("  Fee (bps):          {}", resp.config.fee_bps);
                            println!("  Protocol fee (bps): {}", resp.config.protocol_fee_share_bps);
                            println!("  Max slippage (bps): {}", resp.config.max_slippage_bps);
                            println!("  Imbalance fee (bps):{}", resp.config.imbalance_fee_bps);
                            println!("  Rebate (bps):       {}", resp.config.rebate_bps);
                            println!("  Max swap pct (bps): {}", resp.config.max_swap_pct_bps);
                        } else {
                            println!("Config update failed: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("update-config failed: {}", e);
                    }
                }
            }

            "pending-onramps" => {
                match commands::admin::get_pending_invoice_requests().await {
                    Ok(requests) => {
                        if requests.is_empty() {
                            println!("No pending onramp requests");
                        } else {
                            println!("Pending onramp requests ({}):", requests.len());
                            for req in &requests {
                                println!("  ID: {} | {} sats | recipient: {}", req.request_id, req.amount_sats, req.recipient);
                            }
                        }
                    }
                    Err(e) => {
                        error!("pending-onramps failed: {}", e);
                    }
                }
            }

            "pending-offramps" => {
                match commands::admin::get_pending_offramp_requests().await {
                    Ok(requests) => {
                        if requests.is_empty() {
                            println!("No pending offramp requests");
                        } else {
                            println!("Pending offramp requests ({}):", requests.len());
                            for req in &requests {
                                let amount_sats = req.amount_msat / 1000;
                                println!("  ID: {} | {} sats", req.request_id, amount_sats);
                            }
                        }
                    }
                    Err(e) => {
                        error!("pending-offramps failed: {}", e);
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
                println!("  icp-approve <amount> | Approve ICP for canister (amount in ICP, e.g. 1)");
                println!("  NOTE: Onramp/offramp require 1 ICP anti-DDoS fee (refunded on success)");
                println!("");
                println!("ckBTC Operations:");
                println!("  ckbtc-balance        | Check your ckBTC balance");
                println!("  lp-approve <amount>  | Approve canister to spend ckBTC");
                println!("");
                println!("Onramp (Lightning -> ckBTC):");
                println!("  REQUIRES: icp-approve 1");
                println!("  request-onramp <sats> | Request invoice to receive ckBTC");
                println!("  get-invoice <id>      | Get invoice for request (poll until ready)");
                println!("");
                println!("Offramp (ckBTC -> Lightning):");
                println!("  REQUIRES: icp-approve 1 && lp-approve <ckBTC amount>");
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
                println!("  lp-btc-address       | Get your per-user LP BTC deposit address");
                println!("  lp-btc-deposit [txid]| Claim BTC deposit (after sending to your LP address)");
                println!("  lp-btc-withdraw <amount> <address> | Withdraw BTC from LP");
                println!("");
                println!("Lightning:");
                println!("  ln-address           | Get Lightning address");
                println!("  ln-invoice <amt> <addr> | Create Lightning invoice");
                println!("");
                println!("Relay Registration (for relay operators):");
                println!("  register-relay <pubkey_hex> [webhook_url] [webhook_token] | Register relay");
                println!("  relay-info           | Show registered relay info");
                println!("");
                println!("Rate Limiting:");
                println!("  rate-limit           | Show your rate limit status");
                println!("  (Max 10 onramp + 10 offramp requests per hour)");
                println!("");
                println!("Admin:");
                println!("  whoami               | Show current identity's principal");
                println!("  set-admin <principal> | Set admin (controller-only)");
                println!("  withdraw-fees <principal> | Withdraw protocol fees (admin-only)");
                println!("  update-config [--amp N] [--fee N] [--protocol-fee N] [--slippage N] [--imbalance-fee N] [--rebate N] [--max-swap-pct N]");
                println!("                       | Update StableSwap config (admin-only)");
                println!("  pending-onramps      | List pending onramp invoice requests");
                println!("  pending-offramps     | List pending offramp requests");
                println!("");
                println!("Other:");
                println!("  fetch-key            | Check ckBTC balance");
                println!("  help                 | Show this help");
                println!("  exit/quit            | Stop");
            }
            "exit" | "quit" | "q" => break,
            _ => println!("Unknown command: {}. Type 'help' for available commands.", input),
        }
    }

    info!("CKL CLI stopped");
    Ok(())
}
