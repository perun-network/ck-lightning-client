use anyhow::Result;
use cklightning::ic_types::{
	SignedCandidInvoice,
	RegisterRelayResponse, GetRelayInfoResponse,
	RateLimitStatus,
	UpdateStableSwapConfigRequest, UpdateStableSwapConfigResponse,
	WithdrawProtocolFeesResponse,
	PendingInvoiceRequest, PendingOfframpRequest,
};
use ic_agent::{Identity, AgentError};
use ic_agent::export::Principal;
use ic_lightning_relay::{ICAgent, create_identity, str_home_from_path};
use log::info;

use super::super::get_pem_path;

/// Check ckBTC balance (fetch-key command)
pub(crate) async fn check_ckbtc_balance() -> Result<String> {
	info!("Fetching ckBTC balance");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let str_user = str_home_from_path(get_pem_path());
	let usr_user_id = create_identity(Some(&str_user));
	let usr_user_pr = usr_user_id.sender()
		.map_err(|e| AgentError::MessageError(format!("Failed to get principal from identity: {}", e)))?;

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

	let resp = agent.register_relay(node_pubkey, None, None).await?;

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

/// Get the current identity's principal
pub(crate) async fn whoami() -> Result<String, Box<dyn std::error::Error>> {
	let str_user = str_home_from_path(get_pem_path());
	let identity = create_identity(Some(&str_user));
	let principal = identity.sender()
		.map_err(|e| format!("Failed to get principal: {}", e))?;
	Ok(principal.to_text())
}

/// Set the admin principal (controller-only)
pub(crate) async fn set_admin(principal_text: String) -> Result<(), Box<dyn std::error::Error>> {
	let principal = Principal::from_text(&principal_text)
		.map_err(|e| format!("Invalid principal: {}", e))?;

	info!("Setting admin to: {}", principal_text);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	agent.set_admin(principal).await?;

	info!("Admin set successfully");
	Ok(())
}

/// Withdraw accumulated protocol fees (admin-only)
pub(crate) async fn withdraw_protocol_fees(
	recipient_text: String,
) -> Result<WithdrawProtocolFeesResponse, Box<dyn std::error::Error>> {
	let recipient = Principal::from_text(&recipient_text)
		.map_err(|e| format!("Invalid principal: {}", e))?;

	info!("Withdrawing protocol fees to: {}", recipient_text);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.withdraw_protocol_fees(recipient).await?;

	info!("Withdraw complete: success={}", resp.success);
	Ok(resp)
}

/// Update StableSwap config (admin-only)
pub(crate) async fn update_stableswap_config(
	amplification: Option<u64>,
	fee_bps: Option<u64>,
	protocol_fee_share_bps: Option<u64>,
	max_slippage_bps: Option<u64>,
	imbalance_fee_bps: Option<u64>,
	rebate_bps: Option<u64>,
	max_swap_pct_bps: Option<u64>,
) -> Result<UpdateStableSwapConfigResponse, Box<dyn std::error::Error>> {
	let request = UpdateStableSwapConfigRequest {
		amplification,
		fee_bps,
		protocol_fee_share_bps,
		max_slippage_bps,
		imbalance_fee_bps,
		rebate_bps,
		max_swap_pct_bps,
	};

	info!("Updating StableSwap config: {:?}", request);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.update_stableswap_config(request).await?;

	info!("Config update complete: success={}", resp.success);
	Ok(resp)
}

/// Get pending onramp invoice requests
pub(crate) async fn get_pending_invoice_requests() -> Result<Vec<PendingInvoiceRequest>, Box<dyn std::error::Error>> {
	info!("Fetching pending invoice requests");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let requests = agent.get_pending_invoice_requests().await?;

	info!("Got {} pending invoice requests", requests.len());
	Ok(requests)
}

/// Get pending offramp requests
pub(crate) async fn get_pending_offramp_requests() -> Result<Vec<PendingOfframpRequest>, Box<dyn std::error::Error>> {
	info!("Fetching pending offramp requests");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let requests = agent.get_pending_offramp_requests().await?;

	info!("Got {} pending offramp requests", requests.len());
	Ok(requests)
}
