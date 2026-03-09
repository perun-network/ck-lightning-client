use candid::Nat;
use ck_lightning_client::CKLIGHTNING_LEDGER_ID;
use cklightning::ic_types::{
	LpBalanceResponse, LpDepositResponse, LpWithdrawResponse, TotalLpBalanceResponse,
	LpBtcAddressResponse, LpBtcDepositResponse, LpBtcWithdrawResponse,
};
use ic_agent::export::Principal;
use ic_lightning_relay::{ICAgent, str_home_from_path};
use log::info;

use super::super::get_pem_path;

/// Approve the ckLightning canister to spend caller's ckBTC (ICRC-2)
pub(crate) async fn lp_approve(amount: u64) -> Result<Nat, Box<dyn std::error::Error>> {
	info!("Approving {} satoshis for LP canister", amount);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let can_ckl_id = Principal::from_text(CKLIGHTNING_LEDGER_ID)?;
	let block_idx = agent.tx_icrc2_approve(can_ckl_id, amount).await?;

	info!("Approval successful, block index: {}", block_idx);
	Ok(block_idx)
}

/// Deposit ckBTC to the liquidity pool
pub(crate) async fn lp_deposit(amount: u64) -> Result<LpDepositResponse, Box<dyn std::error::Error>> {
	info!("Depositing {} satoshis to LP", amount);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.deposit_ckbtc(amount).await?;

	info!("Deposit complete: success={}", resp.success);
	Ok(resp)
}

/// Withdraw ckBTC from the liquidity pool
pub(crate) async fn lp_withdraw(amount: u64) -> Result<LpWithdrawResponse, Box<dyn std::error::Error>> {
	info!("Withdrawing {} satoshis from LP", amount);

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.withdraw_ckbtc(amount).await?;

	info!("Withdraw complete: success={}", resp.success);
	Ok(resp)
}

/// Get caller's LP balance
pub(crate) async fn lp_balance() -> Result<LpBalanceResponse, Box<dyn std::error::Error>> {
	info!("Fetching LP balance");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_my_lp_balance().await?;

	info!("LP balance fetched");
	Ok(resp)
}

/// Get total LP balance
pub(crate) async fn lp_total() -> Result<TotalLpBalanceResponse, Box<dyn std::error::Error>> {
	info!("Fetching total LP balance");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_total_lp_balance().await?;

	info!("Total LP balance fetched");
	Ok(resp)
}

/// Get the caller's per-user LP BTC deposit address
pub(crate) async fn lp_btc_address() -> Result<LpBtcAddressResponse, Box<dyn std::error::Error>> {
	info!("Fetching per-user LP BTC address");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.get_lp_btc_user_address().await?;

	info!("LP BTC user address fetched");
	Ok(resp)
}

/// Claim a BTC deposit to the liquidity pool (per-user address)
pub(crate) async fn lp_btc_deposit(txid: Option<Vec<u8>>) -> Result<LpBtcDepositResponse, Box<dyn std::error::Error>> {
	info!("Claiming BTC deposit (per-user address)");

	let agent = ICAgent::new_from_pem_file(Some(str_home_from_path(get_pem_path())))?;
	agent.fetch_root_key().await?;

	let resp = agent.deposit_btc_user(txid, 0).await?;

	info!("BTC deposit claim complete: success={}", resp.success);
	Ok(resp)
}

/// Withdraw BTC from the liquidity pool
pub(crate) async fn lp_btc_withdraw(
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
