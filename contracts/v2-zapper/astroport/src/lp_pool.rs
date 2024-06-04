use std::str::FromStr;

use apollo_cw_asset::{Asset, AssetInfo, AssetInfoBase, AssetList};
use apollo_utils::assets::assert_only_native_coins;
use astroport_v5::{
    asset::{Asset as AstroAsset, AssetInfo as AstroAssetInfo, PairInfo},
    factory::PairType,
    pair::{PoolResponse, QueryMsg as PairQueryMsg, MAX_ALLOWED_SLIPPAGE},
};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, Deps, Env, Event, QuerierWrapper, QueryRequest,
    Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw_dex::{traits::Pool, CwDexError};
use mars_zapper_base::LpPool;

impl LpPool for AstroportLpPool {
    fn get_pool_for_lp_token(
        deps: Deps,
        lp_token_denom: &str,
    ) -> Result<Box<dyn Pool>, CwDexError> {
        let pair_addr = extract_pair_address(&deps, lp_token_denom)?;
        Ok(Self::new(deps, pair_addr).map(|p| {
            let as_trait: Box<dyn Pool> = Box::new(p);
            as_trait
        })?)
    }
}

/// LP token denom structure: `factory/[pair_addr]/astroport/share`
fn extract_pair_address(deps: &Deps, lp_token_denom: &str) -> Result<Addr, CwDexError> {
    let parts: Vec<&str> = lp_token_denom.split('/').collect();
    if parts.len() == 4 && parts[0] == "factory" && parts[2] == "astroport" && parts[3] == "share" {
        Ok(deps.api.addr_validate(parts[1])?)
    } else {
        Err(CwDexError::Std(StdError::generic_err("No pair address found in LP token denom")))
    }
}

/// Represents an AMM pool on Astroport
pub struct AstroportLpPool {
    /// The address of the associated pair contract
    pub pair_addr: Addr,
    /// The LP token denom
    pub lp_token: String,
    /// The assets of the pool
    pub pool_assets: Vec<AstroAssetInfo>,
    /// The type of pool represented: Constant product (*Xyk*) or *Stableswap*
    pub pair_type: PairType,
}

impl AstroportLpPool {
    /// Creates a new instance of `AstroportPool`
    ///
    /// Arguments:
    /// - `pair_addr`: The address of the pair contract associated with the pool
    pub fn new(deps: Deps, pair_addr: Addr) -> StdResult<Self> {
        let pair_info =
            deps.querier.query_wasm_smart::<PairInfo>(pair_addr.clone(), &PairQueryMsg::Pair {})?;

        // Validate pair type. We only support XYK, stable swap, and PCL pools
        match &pair_info.pair_type {
            PairType::Custom(t) => match t.as_str() {
                "concentrated" => Ok(()),
                "astroport-pair-xyk-sale-tax" => Ok(()),
                _ => Err(StdError::generic_err("Custom pair type is not supported")),
            },
            _ => Ok(()),
        }?;

        Ok(Self {
            pair_addr,
            lp_token: pair_info.liquidity_token,
            pool_assets: pair_info.asset_infos,
            pair_type: pair_info.pair_type,
        })
    }

    /// Returns the matching pool given a LP token.
    ///
    /// Arguments:
    /// - `lp_token`: Said LP token
    /// - `pair_addr`: The address of the pair contract associated with the pool
    pub fn get_pool_for_lp_token(
        deps: Deps,
        lp_token: &AstroAssetInfo,
        pair_addr: Addr,
    ) -> Result<Self, CwDexError> {
        match lp_token {
            AstroAssetInfo::NativeToken {
                denom: _,
            } => {
                let pool = AstroportLpPool::new(deps, pair_addr)?;
                Ok(pool)
            }
            AstroAssetInfo::Token {
                contract_addr: _,
            } => Err(CwDexError::NotLpToken {}),
        }
    }

    /// Returns the total supply of the associated LP token
    pub fn query_lp_token_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        let coin = querier.query_supply(self.lp_token.clone())?;
        Ok(coin.amount)
    }

    /// Queries the pair contract for the current pool state
    pub fn query_pool_info(&self, querier: &QuerierWrapper) -> StdResult<PoolResponse> {
        querier.query::<PoolResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.pair_addr.to_string(),
            msg: to_json_binary(&PairQueryMsg::Pool {})?,
        }))
    }
}

impl Pool for AstroportLpPool {
    fn provide_liquidity(
        &self,
        _deps: Deps,
        _env: &Env,
        assets: AssetList,
        min_out: Uint128,
    ) -> Result<Response, CwDexError> {
        // all assets are native
        let mut coins = assert_only_native_coins(&assets)?;

        // sort coins
        coins.sort_by(|a, b| a.denom.to_string().cmp(&b.denom));

        let astro_assets: Vec<AstroAsset> = coins.iter().map(|c| c.into()).collect();

        // Create the provide liquidity message
        let provide_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.pair_addr.to_string(),
            msg: to_json_binary(&astroport_v5::pair::ExecuteMsg::ProvideLiquidity {
                assets: astro_assets,
                slippage_tolerance: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
                auto_stake: Some(false),
                receiver: None,
                min_lp_to_receive: Some(min_out),
            })?,
            funds: coins,
        });

        let event = Event::new("provide_liquidity")
            .add_attribute("pair_addr", &self.pair_addr)
            .add_attribute("assets", format!("{:?}", assets));

        Ok(Response::new().add_message(provide_liquidity_msg).add_event(event))
    }

    fn withdraw_liquidity(
        &self,
        _deps: Deps,
        _env: &Env,
        asset: Asset,
        min_out: AssetList,
    ) -> Result<Response, CwDexError> {
        if let AssetInfoBase::Native(denom) = &asset.info {
            let min_out_coins = assert_only_native_coins(&min_out)?;
            let astro_assets: Vec<AstroAsset> = min_out_coins.iter().map(|c| c.into()).collect();

            let withdraw_liquidity = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.pair_addr.to_string(),
                msg: to_json_binary(&astroport_v5::pair::ExecuteMsg::WithdrawLiquidity {
                    // This field is currently not used...
                    assets: vec![],
                    min_assets_to_receive: Some(astro_assets),
                })?,
                funds: vec![Coin::new(asset.amount.u128(), denom)],
            });

            let event = Event::new("withdraw_liquidity")
                .add_attribute("pair_addr", &self.pair_addr)
                .add_attribute("asset", format!("{:?}", asset))
                .add_attribute("token_amount", asset.amount);

            Ok(Response::new().add_message(withdraw_liquidity).add_event(event))
        } else {
            Err(CwDexError::InvalidInAsset {
                a: asset,
            })
        }
    }

    fn swap(
        &self,
        _deps: Deps,
        _env: &Env,
        _offer_asset: Asset,
        _ask_asset_info: AssetInfo,
        _min_out: Uint128,
    ) -> Result<Response, CwDexError> {
        unimplemented!("use cw-dex dependency instead")
    }

    fn get_pool_liquidity(&self, deps: Deps) -> Result<AssetList, CwDexError> {
        let resp = self.query_pool_info(&deps.querier)?;
        Ok(from_astro_to_apollo_assets(&resp.assets))
    }

    fn simulate_provide_liquidity(
        &self,
        deps: Deps,
        _env: &Env,
        assets: AssetList,
    ) -> Result<Asset, CwDexError> {
        let coins = assert_only_native_coins(&assets)?;
        let astro_assets: Vec<AstroAsset> = coins.iter().map(|c| c.into()).collect();

        let amount: Uint128 = deps.querier.query_wasm_smart(
            self.pair_addr.to_string(),
            &astroport_v5::pair::QueryMsg::SimulateProvide {
                assets: astro_assets,
                slippage_tolerance: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
            },
        )?;

        let lp_token = Asset {
            info: AssetInfo::Native(self.lp_token.clone()),
            amount,
        };

        Ok(lp_token)
    }

    fn simulate_withdraw_liquidity(
        &self,
        deps: Deps,
        lp_token: &Asset,
    ) -> Result<AssetList, CwDexError> {
        let assets: Vec<AstroAsset> = deps.querier.query_wasm_smart(
            self.pair_addr.to_string(),
            &astroport_v5::pair::QueryMsg::SimulateWithdraw {
                lp_amount: lp_token.amount,
            },
        )?;

        Ok(from_astro_to_apollo_assets(&assets))
    }

    fn simulate_swap(
        &self,
        _deps: Deps,
        _offer_asset: Asset,
        _ask_asset_info: AssetInfo,
    ) -> StdResult<Uint128> {
        unimplemented!("use cw-dex dependency instead")
    }

    fn lp_token(&self) -> AssetInfo {
        AssetInfoBase::Native(self.lp_token.clone())
    }

    fn pool_assets(&self, _deps: Deps) -> StdResult<Vec<AssetInfo>> {
        Ok(from_astro_to_apollo_assets_info(&self.pool_assets))
    }
}

fn from_astro_to_apollo_assets(assets: &Vec<AstroAsset>) -> AssetList {
    let mut asset_list = AssetList::default();
    for a in assets {
        // Add can fail only if we have duplicated assets.
        // Save to unwrap because assets are not duplicated (error should never happen).
        asset_list
            .add(&Asset {
                info: from_astro_to_apollo_asset_info(a.info.clone()),
                amount: a.amount,
            })
            .unwrap();
    }

    asset_list
}

fn from_astro_to_apollo_assets_info(assets: &[AstroAssetInfo]) -> Vec<AssetInfo> {
    assets.iter().map(|a| from_astro_to_apollo_asset_info(a.clone())).collect()
}

/// Converts Astro to Apollo `AssetInfo`. Apollo crates have coverters from Astro to Apollo but we use
/// different astroport version so we can't use them directly.
fn from_astro_to_apollo_asset_info(asset_info: AstroAssetInfo) -> AssetInfo {
    match asset_info {
        AstroAssetInfo::NativeToken {
            denom,
        } => AssetInfo::Native(denom),
        AstroAssetInfo::Token {
            contract_addr,
        } => AssetInfo::Cw20(contract_addr),
    }
}
