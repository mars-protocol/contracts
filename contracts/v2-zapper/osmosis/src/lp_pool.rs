use std::str::FromStr;

use cosmwasm_std::Deps;
use cw_dex::{osmosis::OsmosisPool, traits::Pool, CwDexError};
use mars_zapper_base::LpPool;

pub struct OsmosisLpPool {}

impl OsmosisLpPool {
    /// Returns the matching pool given a LP token.
    ///
    /// Based on impl from https://github.com/apollodao/cw-dex/blob/develop/src/implementations/pool.rs#L60
    pub fn get_pool_for_lp_token(
        deps: Deps,
        lp_token_denom: &str,
    ) -> Result<OsmosisPool, CwDexError> {
        // The only Pool implementation that uses native denoms right now is Osmosis
        if !lp_token_denom.starts_with("gamm/pool/") {
            return Err(CwDexError::NotLpToken {});
        }

        let pool_id_str =
            lp_token_denom.strip_prefix("gamm/pool/").ok_or(CwDexError::NotLpToken {})?;

        let pool_id = u64::from_str(pool_id_str).map_err(|_| CwDexError::NotLpToken {})?;

        Ok(OsmosisPool::new(pool_id, deps)?)
    }
}

impl LpPool for OsmosisLpPool {
    fn get_pool_for_lp_token(
        deps: Deps,
        lp_token_denom: &str,
    ) -> Result<Box<dyn Pool>, CwDexError> {
        Self::get_pool_for_lp_token(deps, lp_token_denom).map(|p| {
            let as_trait: Box<dyn Pool> = Box::new(p);
            as_trait
        })
    }
}
