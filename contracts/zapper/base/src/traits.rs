use cosmwasm_std::Deps;
use cw_dex::traits::Pool;
use cw_dex::CwDexError;

pub trait LpPool {
    /// Returns the matching pool given a LP token.
    ///
    /// https://github.com/apollodao/cw-dex uses cargo feature flags for chain specific implementation.
    fn get_pool_for_lp_token(deps: Deps, lp_token_denom: &str)
        -> Result<Box<dyn Pool>, CwDexError>;
}
