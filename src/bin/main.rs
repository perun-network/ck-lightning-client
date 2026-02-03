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

            "help" | "h" => {
                println!("=== ckLightning Client Commands ===");
                println!("");
                println!("User BTC (threshold ECDSA):");
                println!("  btc-address          | Get your BTC address (derived from principal)");
                println!("  btc-balance          | Check your BTC balance");
                println!("  btc-send <amt> <addr>| Send BTC from your address");
                println!("");
                println!("ckBTC Operations:");
                println!("  ckbtc-balance        | Check your ckBTC balance");
                println!("  lp-approve <amount>  | Approve canister to spend ckBTC");
                println!("");
                println!("Offramp (ckBTC -> Lightning):");
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
