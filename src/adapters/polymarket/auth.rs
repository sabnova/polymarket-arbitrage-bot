use crate::adapters::polymarket::PolymarketApi;
use anyhow::Result;

pub async fn authenticate(api: &PolymarketApi) -> Result<()> {
    api.authenticate().await
}
