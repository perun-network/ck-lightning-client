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
use ck_lightning_client::{CKLIGHTNING_LEDGER_ID, PEM_USER_ACC_PATH};
use cklightning::ic_types::SignedCandidInvoice;
use clap::{Parser, Subcommand};
use ic_agent::Identity;
use ic_agent::{AgentError, export::Principal};
use ic_ledger_types::{AccountIdentifier, Subaccount};
use ldk_sample::{ICAgent, create_identity, str_home_from_path};
use log::{error, info, warn};
use std::io::{self, BufRead, Write};

#[derive(Subcommand)]
enum Commands {
    /// Test RPC handlers
    Rpc {
        #[arg(short, long)]
        msg: String,
    },
    /// Test P2P handlers  
    P2p {
        #[arg(short, long)]
        msg: String,
        #[arg(short, long, default_value = "test-node-123")]
        remote_id: String,
    },
    /// Show mock endpoints state
    Status,
    // Fetches RootKey
    FetchKey {
        #[arg(short, long, default_value = PEM_USER_ACC_PATH)]
        pem: String,
    },
}

#[derive(Parser)]
#[command(name = "ckl-cli", about = "Test CKL RPC/P2P handlers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    info!("🧪 CKL CLI Started - Interactive Mode");

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

            "help" | "h" => {
                println!("rpc <msg>     | Test RPC");
                println!("p2p <msg> [id]| Test P2P");
                println!("status        | Show status");
                println!("fetch-key     | ICAgent test");
                println!("exit/quit     | Stop");
            }
            "exit" | "quit" | "q" => break,
            _ => println!("? {}", input),
        }
    }

    info!("👋 CKL CLI stopped");
    Ok(())
}

async fn check_ckbtc_balance() -> Result<String> {
    info!("🧪 Fetching root key with PEM: {}", PEM_USER_ACC_PATH);

    // Use your existing ICAgent from ic-lightning-relay
    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(PEM_USER_ACC_PATH)))?;
    agent.fetch_root_key().await?;

    let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID).unwrap();
    println!("ckLightning Ledger Canister ID: {:?}", can_ckl_id);

    let str_user = str_home_from_path(PEM_USER_ACC_PATH);
    let usr_user_id = create_identity(Some(&str_user));
    let usr_user_pr = usr_user_id.sender().unwrap();
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

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(PEM_USER_ACC_PATH)))?;
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

    let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(PEM_USER_ACC_PATH)))?;
    agent.fetch_root_key().await?;

    // Create CkLightningClient
    let client = ck_lightning_client::CkLightningClient::new(agent);

    let signed_invoice = client.query_ln_invoice(amount_msat, btc_address).await?;

    info!("✅ Got signed invoice: {} msat", amount_msat);
    Ok(signed_invoice)
}
