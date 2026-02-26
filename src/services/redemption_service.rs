use crate::adapters::polymarket::PolymarketApi;
use crate::config::Config;
use anyhow::Result;
use log::{info, warn};
use std::sync::Arc;

pub async fn auto_redeem_winners(
    api: Arc<PolymarketApi>,
    config: &Config,
    redeem_targets: &[(String, String)],
) -> Result<()> {
    if !config.strategy.auto_redeem || config.strategy.simulation_mode {
        return Ok(());
    }
    if config.polymarket.proxy_wallet_address.is_none() {
        return Ok(());
    }

    for (condition_id, outcome) in redeem_targets {
        if let Err(e) = api.redeem_tokens(condition_id, "", outcome).await {
            warn!("Redeem failed for {} {}: {}", condition_id, outcome, e);
        } else {
            info!("Redeemed {} outcome {} tokens", condition_id, outcome);
        }
    }
    Ok(())
}
