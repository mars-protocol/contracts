use std::str::FromStr;

use apollo_cw_asset::{Asset, AssetInfo, AssetInfoBase, AssetList};
use apollo_utils::{assets::assert_only_native_coins, iterators::IntoElementwise};
use astroport::{
    asset::{Asset as AstroAsset, AssetInfo as AstroAssetInfo, PairInfo},
    factory::PairType,
    liquidity_manager,
    pair::{
        Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as PairExecuteMsg, PoolResponse,
        QueryMsg as PairQueryMsg, SimulationResponse, MAX_ALLOWED_SLIPPAGE,
    },
    querier::query_supply,
};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, Deps, Env, Event, QuerierWrapper, QueryRequest,
    Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw_dex::{traits::Pool, CwDexError};

/// Represents an AMM pool on Astroport
pub struct AstroportPool {
    /// The address of the associated pair contract
    pub pair_addr: Addr,
    /// The address of the associated LP token contract
    pub lp_token_addr: Addr,
    /// The assets of the pool
    pub pool_assets: Vec<AstroAssetInfo>,
    /// The type of pool represented: Constant product (*Xyk*) or *Stableswap*
    pub pair_type: PairType,
    /// The address of the Astroport liquidity manager contract
    pub liquidity_manager: Addr,
}

impl AstroportPool {
    /// Creates a new instance of `AstroportPool`
    ///
    /// Arguments:
    /// - `pair_addr`: The address of the pair contract associated with the pool
    pub fn new(deps: Deps, pair_addr: Addr, liquidity_manager: Addr) -> StdResult<Self> {
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
            lp_token_addr: pair_info.liquidity_token,
            pool_assets: pair_info.asset_infos,
            pair_type: pair_info.pair_type,
            liquidity_manager,
        })
    }

    /// Returns the matching pool given a LP token.
    ///
    /// Arguments:
    /// - `lp_token`: Said LP token
    /// - `pair_addr`: The address of the pair contract associated with the pool
    /// - `astroport_liquidity_manager`: The Astroport liquidity manager
    ///   address.
    pub fn get_pool_for_lp_token(
        deps: Deps,
        lp_token: &AstroAssetInfo,
        pair_addr: Addr,
        astroport_liquidity_manager: Addr,
    ) -> Result<Self, CwDexError> {
        match lp_token {
            AstroAssetInfo::NativeToken {
                denom,
            } => {
                let pool = AstroportPool::new(deps, pair_addr, astroport_liquidity_manager)?;
                Ok(pool)
            }
            AstroAssetInfo::Token {
                contract_addr,
            } => Err(CwDexError::NotLpToken {}),
        }
    }

    /// Returns the total supply of the associated LP token
    pub fn query_lp_token_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        query_supply(querier, self.lp_token_addr.to_owned())
    }

    /// Queries the pair contract for the current pool state
    pub fn query_pool_info(&self, querier: &QuerierWrapper) -> StdResult<PoolResponse> {
        querier.query::<PoolResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.pair_addr.to_string(),
            msg: to_json_binary(&PairQueryMsg::Pool {})?,
        }))
    }
}

impl Pool for AstroportPool {
    fn provide_liquidity(
        &self,
        _deps: Deps,
        env: &Env,
        assets: AssetList,
        min_out: Uint128,
    ) -> Result<Response, CwDexError> {
        // all assets are native
        let mut coins = assert_only_native_coins(&assets)?;

        // sort coins
        coins.sort_by(|a, b| a.denom.to_string().cmp(&b.denom));

        let astro_assets: Vec<AstroAsset> = assets.clone().into();

        // Create the provide liquidity message
        let provide_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.liquidity_manager.to_string(),
            msg: to_json_binary(&liquidity_manager::ExecuteMsg::ProvideLiquidity {
                pair_addr: self.pair_addr.to_string(),
                min_lp_to_receive: Some(min_out),
                pair_msg: astroport::pair::ExecuteMsg::ProvideLiquidity {
                    assets: astro_assets,
                    slippage_tolerance: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
                    auto_stake: Some(false),
                    receiver: None,
                },
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
        mut min_out: AssetList,
    ) -> Result<Response, CwDexError> {
        if let AssetInfoBase::Native(denom) = &asset.info {
            let min_out_coins = assert_only_native_coins(&min_out)?;

            let withdraw_liquidity = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.liquidity_manager.to_string(),
                msg: to_json_binary(&liquidity_manager::Cw20HookMsg::WithdrawLiquidity {
                    pair_msg: astroport::pair::Cw20HookMsg::WithdrawLiquidity {
                        // This field is currently not used...
                        assets: vec![],
                    },
                    min_assets_to_receive: min_out.to_vec().into_elementwise(),
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
        env: &Env,
        offer_asset: Asset,
        ask_asset_info: AssetInfo,
        min_out: Uint128,
    ) -> Result<Response, CwDexError> {
        unimplemented!("use cw-dex dependency instead")
    }

    fn get_pool_liquidity(&self, deps: Deps) -> Result<AssetList, CwDexError> {
        let resp = self.query_pool_info(&deps.querier)?;
        Ok(resp.assets.to_vec().into())
    }

    fn simulate_provide_liquidity(
        &self,
        deps: Deps,
        _env: &Env,
        assets: AssetList,
    ) -> Result<Asset, CwDexError> {
        let amount: Uint128 = deps.querier.query_wasm_smart(
            self.liquidity_manager.to_string(),
            &liquidity_manager::QueryMsg::SimulateProvide {
                pair_addr: self.pair_addr.to_string(),
                pair_msg: astroport::pair::ExecuteMsg::ProvideLiquidity {
                    assets: assets.into(),
                    slippage_tolerance: Some(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?),
                    auto_stake: Some(false),
                    receiver: None,
                },
            },
        )?;

        let lp_token = Asset {
            info: AssetInfo::Cw20(self.lp_token_addr.clone()),
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
            self.liquidity_manager.to_string(),
            &liquidity_manager::QueryMsg::SimulateWithdraw {
                pair_addr: self.pair_addr.to_string(),
                lp_tokens: lp_token.amount,
            },
        )?;

        Ok(assets.into())
    }

    fn simulate_swap(
        &self,
        deps: Deps,
        offer_asset: Asset,
        ask_asset_info: AssetInfo,
    ) -> StdResult<Uint128> {
        unimplemented!("use cw-dex dependency instead")
    }

    fn lp_token(&self) -> AssetInfo {
        AssetInfoBase::Cw20(self.lp_token_addr.clone())
    }

    fn pool_assets(&self, _deps: Deps) -> StdResult<Vec<AssetInfo>> {
        // TODO: fix this
        // Ok(self.pool_assets.clone())
        unimplemented!("")
    }
}
