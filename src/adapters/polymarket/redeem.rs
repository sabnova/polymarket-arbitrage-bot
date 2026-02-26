use crate::adapters::polymarket::PolymarketApi;
use crate::models::RedeemResponse;
use anyhow::Result;

pub async fn get_redeemable_positions(api: &PolymarketApi, wallet: &str) -> Result<Vec<String>> {
    api.get_redeemable_positions(wallet).await
}

pub async fn redeem_tokens(
    api: &PolymarketApi,
    condition_id: &str,
    token_id: &str,
    outcome: &str,
) -> Result<RedeemResponse> {
    api.redeem_tokens(condition_id, token_id, outcome).await
}
