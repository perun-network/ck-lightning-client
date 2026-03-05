use candid::Nat;
use ck_lightning_client::CKLIGHTNING_LEDGER_ID;
use cklightning::ic_types::{
	OfframpResponse, GetOfframpStatusResponse,
	OnrampInvoiceResponse, GetInvoiceResponse,
};
use ic_agent::Identity;
use ic_agent::export::Principal;
use ic_lightning_relay::{ICAgent, create_identity, str_home_from_path};
use log::info;

use super::super::get_pem_path;

/// Get user's on-chain ckBTC balance
pub(crate) async fn get_user_ckbtc_balance() -> Result<Nat, Box<dyn std::error::Error>> {
	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let str_user = str_home_from_path(get_pem_path());
	let usr_user_id = create_identity(Some(&str_user));
	let usr_user_pr = usr_user_id.sender()?;

	let balance = agent.icrc1_balance_of(usr_user_pr).await?;
	Ok(balance)
}

/// Get user's ICP balance
pub(crate) async fn get_user_icp_balance() -> Result<Nat, Box<dyn std::error::Error>> {
	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let str_user = str_home_from_path(get_pem_path());
	let usr_user_id = create_identity(Some(&str_user));
	let usr_user_pr = usr_user_id.sender()?;

	let balance = agent.icp_balance_of(usr_user_pr).await?;
	Ok(balance)
}

/// Approve the ckLightning canister to spend caller's ICP (ICRC-2)
pub(crate) async fn icp_approve(amount_e8s: u64) -> Result<Nat, Box<dyn std::error::Error>> {
	info!("Approving {} e8s ICP for canister", amount_e8s);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID)?;
	let block_idx = agent.tx_icp_icrc2_approve(can_ckl_id, amount_e8s).await?;

	info!("ICP approval successful, block index: {}", block_idx);
	Ok(block_idx)
}

/// Request an onramp invoice (Lightning -> ckBTC)
pub(crate) async fn request_onramp_invoice(
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
pub(crate) async fn get_invoice(
	request_id: String,
) -> Result<GetInvoiceResponse, Box<dyn std::error::Error>> {
	info!("Fetching invoice for request: {}", request_id);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_invoice(request_id).await?;

	info!("Invoice status fetched");
	Ok(resp)
}

/// Request an offramp (ckBTC -> Lightning)
pub(crate) async fn request_offramp(
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
pub(crate) async fn get_offramp_status(
	request_id: String,
) -> Result<GetOfframpStatusResponse, Box<dyn std::error::Error>> {
	info!("Fetching offramp status for: {}", request_id);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_offramp_status(request_id).await?;

	info!("Offramp status fetched");
	Ok(resp)
}
