use cklightning::ic_types::{
	DepositorBtcBalanceResponse, SendFromDepositorResponse,
};
use ic_lightning_relay::{ICAgent, str_home_from_path};
use log::info;

use super::super::get_pem_path;

/// Get the caller's BTC address (derived from principal via threshold ECDSA)
pub(crate) async fn get_depositor_btc_address() -> Result<DepositorBtcBalanceResponse, Box<dyn std::error::Error>> {
	info!("Fetching depositor BTC address");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_depositor_btc_balance().await?;

	info!("Depositor BTC address fetched");
	Ok(resp)
}

/// Get the caller's BTC balance at their depositor address
pub(crate) async fn get_depositor_btc_balance() -> Result<DepositorBtcBalanceResponse, Box<dyn std::error::Error>> {
	info!("Fetching depositor BTC balance");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_depositor_btc_balance().await?;

	info!("Depositor BTC balance fetched: {} sats", resp.balance_sat);
	Ok(resp)
}

/// Send BTC from the caller's depositor address to a destination
pub(crate) async fn send_btc_from_depositor(
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
