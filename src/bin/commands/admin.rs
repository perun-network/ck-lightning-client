use anyhow::Result;
use ck_lightning_client::CKLIGHTNING_LEDGER_ID;
use cklightning::ic_types::{
	SignedCandidInvoice,
	RegisterRelayResponse, GetRelayInfoResponse,
	RateLimitStatus,
};
use ic_agent::{Identity, AgentError};
use ic_agent::export::Principal;
use ic_ledger_types::{AccountIdentifier, Subaccount};
use ldk_sample::{ICAgent, create_identity, str_home_from_path};
use log::info;

use super::super::get_pem_path;

/// Check ckBTC balance (fetch-key command)
pub(crate) async fn check_ckbtc_balance() -> Result<String> {
	info!("Fetching root key with PEM: {}", get_pem_path());

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

	let resp_user_balance = agent
		.icrc1_balance_of(usr_user_pr)
		.await
		.map_err(|e| AgentError::MessageError(format!("Failed to get user balance: {}", e)))?;

	Ok(format!("User Balance: {}", resp_user_balance))
}

/// Get Lightning address
pub(crate) async fn get_ln_address_cli() -> Result<String, Box<dyn std::error::Error>> {
	info!("Requesting LN Address");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let client = ck_lightning_client::CkLightningClient::new(agent);
	let ln_address = client.get_ln_address().await?;

	info!("Got LN address");
	Ok(ln_address)
}

/// Get Lightning Invoice
pub(crate) async fn get_ln_invoice(
	amount_msat: u64,
	btc_address: String,
) -> Result<SignedCandidInvoice, Box<dyn std::error::Error>> {
	info!(
		"Requesting LN Invoice: {} msat -> {}",
		amount_msat, btc_address
	);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let client = ck_lightning_client::CkLightningClient::new(agent);

	let signed_invoice = client.query_ln_invoice(amount_msat, btc_address).await?;

	info!("Got signed invoice: {} msat", amount_msat);
	Ok(signed_invoice)
}

/// Register a relay with its Lightning node pubkey
pub(crate) async fn register_relay(node_pubkey: Vec<u8>) -> Result<RegisterRelayResponse, Box<dyn std::error::Error>> {
	info!("Registering relay with node pubkey: {}", hex::encode(&node_pubkey));

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.register_relay(node_pubkey).await?;

	info!("Relay registration complete: success={}", resp.success);
	Ok(resp)
}

/// Get information about the registered relay
pub(crate) async fn get_relay_info() -> Result<GetRelayInfoResponse, Box<dyn std::error::Error>> {
	info!("Fetching relay info");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_relay_info().await?;

	info!("Relay info fetched: registered={}", resp.registered);
	Ok(resp)
}

/// Get the caller's rate limit status
pub(crate) async fn get_rate_limit_status() -> Result<RateLimitStatus, Box<dyn std::error::Error>> {
	info!("Fetching rate limit status");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_rate_limit_status().await?;

	info!("Rate limit status fetched");
	Ok(resp)
}

/// Set test timeouts for swap expiry (for E2E testing)
pub(crate) async fn set_test_timeouts(
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
pub(crate) async fn check_expired_swaps() -> Result<(), Box<dyn std::error::Error>> {
	info!("Triggering expired swap check");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	agent.check_expired_swaps().await?;

	info!("Expired swap check complete");
	Ok(())
}

/// Get expired swap counts
pub(crate) async fn get_expired_swap_counts() -> Result<(u64, u64), Box<dyn std::error::Error>> {
	info!("Fetching expired swap counts");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let (onramp, offramp) = agent.get_expired_swap_counts().await?;

	info!("Expired counts: onramp={}, offramp={}", onramp, offramp);
	Ok((onramp, offramp))
}
