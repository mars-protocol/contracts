#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use mars_core::error::MarsError;
use terra_cosmwasm::TerraQuerier;

use mars_core::asset::Asset;
use mars_core::helpers::option_string_to_addr;
use mars_core::math::decimal::Decimal;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ASTROPORT_TWAP_SNAPSHOTS, CONFIG, PRICE_SOURCES};
use crate::{AstroportTwapSnapshot, Config, PriceSourceChecked, PriceSourceUnchecked};

use self::helpers::*;
use astroport::pair::TWAP_PRECISION;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => execute_update_config(deps, env, info, owner),
        ExecuteMsg::SetAsset {
            asset,
            price_source,
        } => execute_set_asset(deps, env, info, asset, price_source),
        ExecuteMsg::RecordTwapSnapshots { assets } => {
            execute_record_twap_snapshots(deps, env, info, assets)
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_set_asset(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    asset: Asset,
    price_source_unchecked: PriceSourceUnchecked,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let (asset_label, asset_reference, _) = asset.get_attributes();
    let price_source = price_source_unchecked.to_checked(deps.api)?;
    PRICE_SOURCES.save(deps.storage, &asset_reference, &price_source)?;

    // for spot and TWAP sources, we must make sure: the astroport pair indicated by `pair_address`
    // consists of UST and the asset of interest
    match &price_source {
        PriceSourceChecked::AstroportSpot { pair_address }
        | PriceSourceChecked::AstroportTwap { pair_address, .. } => {
            assert_astroport_pool_assets(&deps.querier, &asset, pair_address)?;
        }
        _ => (),
    }

    Ok(Response::new()
        .add_attribute("action", "set_asset")
        .add_attribute("asset", asset_label)
        .add_attribute("price_source", price_source_unchecked.to_string()))
}

/// Modified from
/// https://github.com/Uniswap/uniswap-v2-periphery/blob/master/contracts/examples/ExampleOracleSimple.sol
pub fn execute_record_twap_snapshots(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    assets: Vec<Asset>,
) -> Result<Response, ContractError> {
    let timestamp = env.block.time.seconds();
    let mut attrs: Vec<Attribute> = vec![];

    for asset in assets {
        let (asset_label, asset_reference, _) = asset.get_attributes();
        let price_source = PRICE_SOURCES.load(deps.storage, &asset_reference)?;

        // Asset must be configured to use TWAP price source
        let (pair_address, window_size, tolerance) = match price_source {
            PriceSourceChecked::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
            } => (pair_address, window_size, tolerance),
            _ => {
                return Err(ContractError::PriceSourceNotTwap {});
            }
        };

        // Load existing snapshots. If there's none, we initialize an empty vector
        let mut snapshots = ASTROPORT_TWAP_SNAPSHOTS
            .load(deps.storage, &asset_reference)
            .unwrap_or_else(|_| vec![]);

        // A potential attack is to repeatly call `RecordTwapSnapshots` so that `snapshots` becomes a
        // very big vector, so that calculating the average price becomes extremely gas expensive.
        // To deter this, we reject a new snapshot if the most recent snapshot is less than `tolerance`
        // seconds ago.
        if let Some(latest_snapshot) = snapshots.last() {
            if timestamp - latest_snapshot.timestamp < tolerance {
                continue;
            }
        }

        // Query new price data
        let price_cumulative = query_astroport_cumulative_price(&deps.querier, &pair_address)?;

        // Purge snapshots that are too old, i.e. more than (window_size + tolerance) away from the
        // current timestamp. These snapshots will never be used in the future for calculating
        // average prices
        snapshots.retain(|snapshot| timestamp - snapshot.timestamp <= window_size + tolerance);

        snapshots.push(AstroportTwapSnapshot {
            timestamp,
            price_cumulative,
        });

        ASTROPORT_TWAP_SNAPSHOTS.save(deps.storage, &asset_reference, &snapshots)?;

        attrs.extend(vec![
            attr("asset", asset_label),
            attr("price_cumulative", price_cumulative),
        ]);
    }

    Ok(Response::new()
        .add_attribute("action", "record_twap_snapshots")
        .add_attribute("timestamp", timestamp.to_string())
        .add_attributes(attrs))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps, env)?),
        QueryMsg::AssetPriceSource { asset } => {
            to_binary(&query_asset_price_source(deps, env, asset)?)
        }
        QueryMsg::AssetPrice { asset } => {
            to_binary(&query_asset_price(deps, env, asset.get_reference())?)
        }
        QueryMsg::AssetPriceByReference { asset_reference } => {
            to_binary(&query_asset_price(deps, env, asset_reference)?)
        }
    }
}

fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_asset_price_source(deps: Deps, _env: Env, asset: Asset) -> StdResult<PriceSourceChecked> {
    PRICE_SOURCES.load(deps.storage, &asset.get_reference())
}

fn query_asset_price(
    deps: Deps,
    env: Env,
    asset_reference: Vec<u8>,
) -> Result<Decimal, ContractError> {
    let price_source = PRICE_SOURCES.load(deps.storage, &asset_reference)?;

    match price_source {
        PriceSourceChecked::Fixed { price } => Ok(price),

        PriceSourceChecked::Native { denom } => {
            let terra_querier = TerraQuerier::new(&deps.querier);

            // NOTE: Exchange rate returns how much of the quote (second argument) is required to
            // buy one unit of the base_denom (first argument).
            // We want to know how much uusd we need to buy 1 of the target currency
            let asset_prices_query = terra_querier
                .query_exchange_rates(denom, vec!["uusd".to_string()])?
                .exchange_rates
                .pop();

            match asset_prices_query {
                Some(exchange_rate_item) => Ok(exchange_rate_item.exchange_rate.into()),
                None => Err(ContractError::NativePriceNotFound {}),
            }
        }

        // NOTE: Spot price is defined as the amount of UST to be returned when swapping `PROBE_AMOUNT`
        // of the asset of interest, divided by `PROBE_AMOUNT`. In the current implementation,
        // `PROBE_AMOUNT` is set to 1,000,000.
        //
        // For example, for MARS-UST pair, if swapping 1,000,000 umars returns 1,200,000 uusd (return
        // amount plus commission), then 1 MARS = 1.2 UST.
        //
        // Why not just take the quotient of the two assets depths? (E.g. if the pool has 120 UST and
        // 100 MARS, then 1 MARS = 1.2 UST) Because this only works for XYK pools, not StableSwap pools.
        PriceSourceChecked::AstroportSpot { pair_address } => {
            query_astroport_spot_price(&deps.querier, &pair_address)
        }

        PriceSourceChecked::AstroportTwap {
            pair_address,
            window_size,
            tolerance,
        } => {
            let snapshots = ASTROPORT_TWAP_SNAPSHOTS.load(deps.storage, &asset_reference)?;

            // First, query the current TWAP snapshot
            let current_snapshot = AstroportTwapSnapshot {
                timestamp: env.block.time.seconds(),
                price_cumulative: query_astroport_cumulative_price(&deps.querier, &pair_address)?,
            };

            // Find the oldest snapshot whose period from current snapshot is within the tolerable window
            // We do this using a linear search, and quit as soon as we find one; otherwise throw error
            let previous_snapshot = snapshots
                .iter()
                .find(|snapshot| period_diff(&current_snapshot, snapshot, window_size) <= tolerance)
                .ok_or(ContractError::NoSnapshotWithinTolerance {})?;

            // Handle the case if Astroport's cumulative price overflows. In this case, cumulative
            // price warps back to zero, resulting in more recent cum. prices being smaller than
            // earlier ones. (same behavior as in Solidity)
            //
            // Calculations below assumes the cumulative price doesn't overflows more than once during
            // the period, which should always be the case in practice
            let price_delta =
                if current_snapshot.price_cumulative >= previous_snapshot.price_cumulative {
                    current_snapshot.price_cumulative - previous_snapshot.price_cumulative
                } else {
                    current_snapshot
                        .price_cumulative
                        .checked_add(Uint128::MAX - previous_snapshot.price_cumulative)?
                };
            let period = current_snapshot.timestamp - previous_snapshot.timestamp;
            // NOTE: Astroport introduces TWAP precision (https://github.com/astroport-fi/astroport/pull/143).
            // We need to divide the result by price_precision: (price_delta / (time * price_precision)).
            let price_precision = Uint128::from(10_u128.pow(TWAP_PRECISION.into()));
            let price =
                Decimal::from_ratio(price_delta, price_precision.checked_mul(period.into())?);

            Ok(price)
        }

        // The value of each unit of the liquidity token is the total value of pool's two assets
        // divided by the liquidity token's total supply
        //
        // NOTE: Price sources must exist for both assets in the pool
        PriceSourceChecked::AstroportLiquidityToken { pair_address } => {
            let pool = query_astroport_pool(&deps.querier, &pair_address)?;

            let asset0: Asset = (&pool.assets[0].info).into();
            let asset0_price = query_asset_price(deps, env.clone(), asset0.get_reference())?;
            let asset0_value = asset0_price * pool.assets[0].amount;

            let asset1: Asset = (&pool.assets[1].info).into();
            let asset1_price = query_asset_price(deps, env, asset1.get_reference())?;
            let asset1_value = asset1_price * pool.assets[1].amount;

            let price = Decimal::from_ratio(asset0_value + asset1_value, pool.total_share);
            Ok(price)
        }

        PriceSourceChecked::StLuna { hub_address } => {
            let stluna_exchange_rate = query_stluna_exchange_rate(&deps.querier, &hub_address)?;

            let luna_asset = Asset::Native {
                denom: "uluna".to_string(),
            };
            let luna_price = query_asset_price(deps, env, luna_asset.get_reference())?;

            let stluna_price = stluna_exchange_rate.checked_mul(luna_price)?;
            Ok(stluna_price)
        }
    }
}

// HELPERS

mod helpers {
    use cosmwasm_std::{
        to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmQuery,
    };

    use mars_core::asset::Asset;
    use mars_core::math::decimal::Decimal;
    use mars_core::oracle::AstroportTwapSnapshot;

    use crate::error::ContractError;

    use astroport::{
        asset::{Asset as AstroportAsset, AssetInfo as AstroportAssetInfo},
        pair::{
            CumulativePricesResponse, PoolResponse, QueryMsg as AstroportQueryMsg,
            SimulationResponse,
        },
    };
    use mars_core::basset::hub::{QueryMsg, StateResponse};

    const PROBE_AMOUNT: Uint128 = Uint128::new(1_000_000);

    pub fn diff(a: u64, b: u64) -> u64 {
        if a > b {
            a - b
        } else {
            b - a
        }
    }

    /// Calculate how much the period between two TWAP snapshots deviates from the desired window size
    pub fn period_diff(
        snapshot1: &AstroportTwapSnapshot,
        snapshot2: &AstroportTwapSnapshot,
        window_size: u64,
    ) -> u64 {
        diff(diff(snapshot1.timestamp, snapshot2.timestamp), window_size)
    }

    pub fn ust() -> AstroportAssetInfo {
        AstroportAssetInfo::NativeToken {
            denom: "uusd".to_string(),
        }
    }

    /// Assert the astroport pair indicated by `pair_address` consists of UST and `asset`
    pub fn assert_astroport_pool_assets(
        querier: &QuerierWrapper,
        asset: &Asset,
        pair_address: &Addr,
    ) -> Result<(), ContractError> {
        let pool = query_astroport_pool(querier, pair_address)?;
        let asset0: Asset = (&pool.assets[0].info).into();
        let asset1: Asset = (&pool.assets[1].info).into();
        let ust: Asset = (&ust()).into();

        if (asset0 == ust && &asset1 == asset) || (asset1 == ust && &asset0 == asset) {
            Ok(())
        } else {
            Err(ContractError::InvalidPair {})
        }
    }

    pub fn query_astroport_pool(
        querier: &QuerierWrapper,
        pair_address: &Addr,
    ) -> StdResult<PoolResponse> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: pair_address.to_string(),
            msg: to_binary(&AstroportQueryMsg::Pool {})?,
        }))
    }

    pub fn query_astroport_spot_price(
        querier: &QuerierWrapper,
        pair_address: &Addr,
    ) -> Result<Decimal, ContractError> {
        let response: PoolResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: pair_address.to_string(),
            msg: to_binary(&AstroportQueryMsg::Pool {})?,
        }))?;

        // During the configuration of the price source, we have asserted that the pool indeed consists
        // of UST and the asset of interest
        // Here,  we use the one asset in the pool that is *not* UST as `offer_asset` to simulate the swap
        let offer_asset_info = if response.assets[0].info == ust() {
            response.assets[1].info.clone()
        } else {
            response.assets[0].info.clone()
        };
        let offer_asset = AstroportAsset {
            info: offer_asset_info,
            amount: PROBE_AMOUNT,
        };

        let response: SimulationResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_address.to_string(),
                msg: to_binary(&AstroportQueryMsg::Simulation { offer_asset })?,
            }))?;

        Ok(Decimal::from_ratio(
            response.return_amount + response.commission_amount,
            PROBE_AMOUNT,
        ))
    }

    pub fn query_astroport_cumulative_price(
        querier: &QuerierWrapper,
        pair_address: &Addr,
    ) -> StdResult<Uint128> {
        let response: CumulativePricesResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_address.to_string(),
                msg: to_binary(&AstroportQueryMsg::CumulativePrices {})?,
            }))?;

        // during the configuration of the price source, we have asserted that the pool indeed consists
        // of UST and the asset of interest.
        // Here, we return cumulative price of the one asset in the pool that is *not* UST
        let price_cumulative = if response.assets[0].info == ust() {
            response.price1_cumulative_last
        } else {
            response.price0_cumulative_last
        };
        Ok(price_cumulative)
    }

    pub fn query_stluna_exchange_rate(
        querier: &QuerierWrapper,
        hub_address: &Addr,
    ) -> StdResult<Decimal> {
        let response: StateResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: hub_address.to_string(),
            msg: to_binary(&QueryMsg::State {})?,
        }))?;
        Ok(response.stluna_exchange_rate.into())
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use astroport::asset::{Asset as AstroportAsset, AssetInfo, PairInfo};
    use astroport::factory::PairType;
    use astroport::pair::{CumulativePricesResponse, SimulationResponse};
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
    use cosmwasm_std::Decimal as StdDecimal;
    use cosmwasm_std::{from_binary, Addr, OwnedDeps};
    use mars_core::basset::hub::StateResponse;
    use mars_core::testing::{mock_dependencies, mock_env_at_block_time, MarsMockQuerier};

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner"),
        };
        let info = mock_info("owner", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(Addr::unchecked("owner"), config.owner);
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup();

        // only owner can update
        {
            let msg = ExecuteMsg::UpdateConfig {
                owner: Some(String::from("new_owner")),
            };
            let info = mock_info("another_one", &[]);
            let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
            assert_eq!(err, MarsError::Unauthorized {}.into());
        }

        let info = mock_info("owner", &[]);
        // no change
        {
            let msg = ExecuteMsg::UpdateConfig { owner: None };
            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

            let config = CONFIG.load(&deps.storage).unwrap();
            assert_eq!(config.owner, Addr::unchecked("owner"));
        }

        // new owner
        {
            let msg = ExecuteMsg::UpdateConfig {
                owner: Some(String::from("new_owner")),
            };
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let config = CONFIG.load(&deps.storage).unwrap();
            assert_eq!(config.owner, Addr::unchecked("new_owner"));
        }
    }

    #[test]
    fn test_set_asset() {
        let mut deps = th_setup();

        let msg = ExecuteMsg::SetAsset {
            asset: Asset::Native {
                denom: "luna".to_string(),
            },
            price_source: PriceSourceUnchecked::Native {
                denom: "luna".to_string(),
            },
        };
        let info = mock_info("another_one", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, MarsError::Unauthorized {}.into());
    }

    #[test]
    fn test_set_asset_fixed() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset: asset,
            price_source: PriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceChecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128)
            }
        );
    }

    #[test]
    fn test_set_asset_native() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Native {
            denom: String::from("luna"),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset: asset,
            price_source: PriceSourceUnchecked::Native {
                denom: "luna".to_string(),
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceChecked::Native {
                denom: "luna".to_string()
            }
        );
    }

    #[test]
    fn test_set_asset_astroport_spot() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let offer_asset_info = AssetInfo::Token {
            contract_addr: Addr::unchecked("cw20token"),
        };
        let ask_asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        deps.querier.set_astroport_pair(PairInfo {
            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
            contract_addr: Addr::unchecked("pair"),
            liquidity_token: Addr::unchecked("lp"),
            pair_type: PairType::Xyk {},
        });

        let asset = Asset::Cw20 {
            contract_addr: "cw20token".to_string(),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset: asset,
            price_source: PriceSourceUnchecked::AstroportSpot {
                pair_address: "pair".to_string(),
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceChecked::AstroportSpot {
                pair_address: Addr::unchecked("pair")
            }
        );
    }

    #[test]
    fn test_set_asset_astroport_twap() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let offer_asset_info = AssetInfo::Token {
            contract_addr: Addr::unchecked("cw20token"),
        };
        let ask_asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        deps.querier.set_astroport_pair(PairInfo {
            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
            contract_addr: Addr::unchecked("pair"),
            liquidity_token: Addr::unchecked("lp"),
            pair_type: PairType::Xyk {},
        });

        let asset = Asset::Cw20 {
            contract_addr: "cw20token".to_string(),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset: asset,
            price_source: PriceSourceUnchecked::AstroportTwap {
                pair_address: "pair".to_string(),
                window_size: 3600,
                tolerance: 600,
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();

        assert_eq!(
            price_source,
            PriceSourceChecked::AstroportTwap {
                pair_address: Addr::unchecked("pair"),
                window_size: 3600,
                tolerance: 600,
            }
        );
    }

    #[test]
    fn test_set_asset_stluna() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Cw20 {
            contract_addr: String::from("stluna_token"),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset: asset,
            price_source: PriceSourceUnchecked::StLuna {
                hub_address: "stluna_hub_addr".to_string(),
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceChecked::StLuna {
                hub_address: Addr::unchecked("stluna_hub_addr")
            }
        );
    }

    #[test]
    fn test_record_twap_snapshots() {
        let mut deps = th_setup();
        let info = mock_info("anyone", &[]);

        let window_size = 3600;
        let tolerance = 600;

        let asset = Asset::Cw20 {
            contract_addr: "cw20token".to_string(),
        };
        let reference = asset.get_reference();

        // set price source to astroport TWAP
        PRICE_SOURCES
            .save(
                &mut deps.storage,
                reference.as_slice(),
                &PriceSourceChecked::AstroportTwap {
                    pair_address: Addr::unchecked("pair"),
                    window_size,
                    tolerance,
                },
            )
            .unwrap();

        // set cumulative price
        let offer_asset_info = AssetInfo::Token {
            contract_addr: Addr::unchecked("cw20token"),
        };
        let ask_asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        let mut cumulative_prices = CumulativePricesResponse {
            assets: [
                AstroportAsset {
                    info: offer_asset_info,
                    amount: Uint128::zero(),
                },
                AstroportAsset {
                    info: ask_asset_info,
                    amount: Uint128::zero(),
                },
            ],
            total_share: Uint128::zero(),
            price0_cumulative_last: Uint128::zero(), // token
            price1_cumulative_last: Uint128::zero(), // uusd
        };

        // set the cumulative price
        cumulative_prices.price0_cumulative_last = Uint128::new(1_000_000000);
        deps.querier
            .set_astroport_pair_cumulative_prices("pair".to_string(), cumulative_prices.clone());

        // record first snapshot
        let snapshot_time = 100_000;

        let msg = ExecuteMsg::RecordTwapSnapshots {
            assets: vec![asset.clone()],
        };

        let response = execute(
            deps.as_mut(),
            mock_env_at_block_time(snapshot_time),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

        assert_eq!(
            response.attributes,
            vec![
                attr("action", "record_twap_snapshots"),
                attr("timestamp", "100000"),
                attr("asset", "cw20token"),
                attr("price_cumulative", "1000000000"),
            ]
        );

        let snapshots = ASTROPORT_TWAP_SNAPSHOTS
            .load(deps.as_ref().storage, &reference)
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].price_cumulative, Uint128::new(1_000_000000));
        assert_eq!(snapshots[0].timestamp, snapshot_time);

        // update the cumulative price
        cumulative_prices.price0_cumulative_last = Uint128::new(2_000_000000);
        deps.querier
            .set_astroport_pair_cumulative_prices("pair".to_string(), cumulative_prices.clone());

        // try to record a second snapshot within `tolerance` seconds
        let snapshot_too_soon_time = snapshot_time + tolerance - 1;

        let response = execute(
            deps.as_mut(),
            mock_env_at_block_time(snapshot_too_soon_time),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

        assert_eq!(
            response.attributes,
            vec![
                attr("action", "record_twap_snapshots"),
                attr("timestamp", "100599"),
            ]
        );
        assert!(response.events.len() == 0);

        // record a second snapshot
        let second_snapshot_time = snapshot_time + tolerance;

        let response = execute(
            deps.as_mut(),
            mock_env_at_block_time(second_snapshot_time),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

        assert_eq!(
            response.attributes,
            vec![
                attr("action", "record_twap_snapshots"),
                attr("timestamp", "100600"),
                attr("asset", "cw20token"),
                attr("price_cumulative", "2000000000"),
            ]
        );

        let snapshots = ASTROPORT_TWAP_SNAPSHOTS
            .load(deps.as_ref().storage, &reference)
            .unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[1].price_cumulative, Uint128::new(2_000_000000));
        assert_eq!(snapshots[1].timestamp, second_snapshot_time);

        // record a third snapshot and check that old snapshots are removed from state
        let third_snapshot_time = second_snapshot_time + window_size + tolerance + 1;

        execute(
            deps.as_mut(),
            mock_env_at_block_time(third_snapshot_time),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

        let snapshots = ASTROPORT_TWAP_SNAPSHOTS
            .load(deps.as_ref().storage, &reference)
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].price_cumulative, Uint128::new(2_000_000000));
        assert_eq!(snapshots[0].timestamp, third_snapshot_time);
    }

    #[test]
    fn test_query_asset_price_source() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };

        let msg = ExecuteMsg::SetAsset {
            asset: asset.clone(),
            price_source: PriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let price_source: PriceSourceChecked = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceSource { asset },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            price_source,
            PriceSourceChecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        );
    }

    #[test]
    fn test_query_asset_price_fixed() {
        let mut deps = th_setup();
        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };
        let asset_reference = asset.get_reference();

        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::Fixed {
                    price: Decimal::from_ratio(3_u128, 2_u128),
                },
            )
            .unwrap();

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceByReference { asset_reference },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(price, Decimal::from_ratio(3_u128, 2_u128));
    }

    #[test]
    fn test_query_asset_price_native() {
        let mut deps = th_setup();
        let asset = Asset::Native {
            denom: String::from("nativecoin"),
        };
        let asset_reference = asset.get_reference();

        deps.querier.set_native_exchange_rates(
            "nativecoin".to_string(),
            &[("uusd".to_string(), Decimal::from_ratio(4_u128, 1_u128))],
        );

        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::Native {
                    denom: "nativecoin".to_string(),
                },
            )
            .unwrap();

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceByReference { asset_reference },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(price, Decimal::from_ratio(4_u128, 1_u128));
    }

    #[test]
    fn test_query_asset_price_astroport_spot() {
        let mut deps = th_setup();
        let asset = Asset::Native {
            denom: String::from("cw20token"),
        };
        let asset_reference = asset.get_reference();

        // set price source
        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::AstroportSpot {
                    pair_address: Addr::unchecked("pair"),
                },
            )
            .unwrap();

        // set astroport pair info
        let offer_asset_info = AssetInfo::Token {
            contract_addr: Addr::unchecked("cw20token"),
        };
        let ask_asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        deps.querier.set_astroport_pair(PairInfo {
            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
            contract_addr: Addr::unchecked("pair"),
            liquidity_token: Addr::unchecked("lp"),
            pair_type: PairType::Xyk {},
        });

        // set astroport spot price and query it
        deps.querier.set_astroport_pair_simulation(
            "pair".to_string(),
            SimulationResponse {
                return_amount: Uint128::new(9_000000),
                commission_amount: Uint128::new(1_000000),
                spread_amount: Uint128::zero(),
            },
        );

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceByReference {
                    asset_reference: asset_reference.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(price, Decimal::from_ratio(10_u128, 1_u128));
    }

    #[test]
    fn test_query_asset_price_astroport_twap() {
        let mut deps = th_setup();
        let info = mock_info("anyone", &[]);

        let asset = Asset::Native {
            denom: String::from("cw20token"),
        };
        let asset_reference = asset.get_reference();

        let window_size = 3600;
        let tolerance = 600;

        // set price source
        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::AstroportTwap {
                    pair_address: Addr::unchecked("pair"),
                    window_size,
                    tolerance,
                },
            )
            .unwrap();

        // set astroport pair info
        let offer_asset_info = AssetInfo::Token {
            contract_addr: Addr::unchecked("cw20token"),
        };
        let ask_asset_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        let mut cumulative_prices = CumulativePricesResponse {
            assets: [
                AstroportAsset {
                    info: offer_asset_info.clone(),
                    amount: Uint128::zero(),
                },
                AstroportAsset {
                    info: ask_asset_info.clone(),
                    amount: Uint128::zero(),
                },
            ],
            total_share: Uint128::zero(),
            price0_cumulative_last: Uint128::zero(), // token
            price1_cumulative_last: Uint128::zero(), // uusd
        };

        deps.querier.set_astroport_pair(PairInfo {
            asset_infos: [offer_asset_info, ask_asset_info],
            contract_addr: Addr::unchecked("pair"),
            liquidity_token: Addr::unchecked("lp"),
            pair_type: PairType::Xyk {},
        });

        // record snapshot
        let snapshot_time = 100_000;

        let snapshot_time_cumulative_price = 10_000_000000;
        cumulative_prices.price0_cumulative_last = Uint128::new(snapshot_time_cumulative_price);
        deps.querier
            .set_astroport_pair_cumulative_prices("pair".to_string(), cumulative_prices.clone());

        let msg = ExecuteMsg::RecordTwapSnapshots {
            assets: vec![asset.clone()],
        };

        execute(
            deps.as_mut(),
            mock_env_at_block_time(snapshot_time),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

        // query price when no snapshot was taken within the tolerable window
        let query_error_time = snapshot_time + window_size - tolerance - 1;

        let error = query(
            deps.as_ref(),
            mock_env_at_block_time(query_error_time),
            QueryMsg::AssetPriceByReference {
                asset_reference: asset_reference.clone(),
            },
        )
        .unwrap_err();

        assert_eq!(error, ContractError::NoSnapshotWithinTolerance {}.into());

        // query price when a snapshot was taken within the tolerable window
        let query_time = snapshot_time + window_size;

        let query_time_cumulative_price = 20_000_000000;
        cumulative_prices.price0_cumulative_last = Uint128::new(query_time_cumulative_price);
        deps.querier
            .set_astroport_pair_cumulative_prices("pair".to_string(), cumulative_prices.clone());

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env_at_block_time(query_time),
                QueryMsg::AssetPriceByReference {
                    asset_reference: asset_reference.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            price,
            Decimal::from_ratio(
                query_time_cumulative_price - snapshot_time_cumulative_price,
                (query_time - snapshot_time) * 10_u64.pow(TWAP_PRECISION.into())
            )
        );
    }

    #[test]
    fn test_query_asset_price_stluna() {
        let mut deps = th_setup();

        // Setup Luna native price source
        let asset = Asset::Native {
            denom: "uluna".to_string(),
        };
        let asset_reference = asset.get_reference();

        deps.querier.set_native_exchange_rates(
            "uluna".to_string(),
            &[("uusd".to_string(), Decimal::from_ratio(94_u128, 1_u128))],
        );

        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::Native {
                    denom: "uluna".to_string(),
                },
            )
            .unwrap();

        // Setup stLuna (stLuna / Luna) price source
        let asset = Asset::Cw20 {
            contract_addr: String::from("stluna_token"),
        };
        let asset_reference = asset.get_reference();

        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::StLuna {
                    hub_address: Addr::unchecked("stluna_hub_addr"),
                },
            )
            .unwrap();

        deps.querier.set_basset_state_response(StateResponse {
            bluna_exchange_rate: Default::default(),
            stluna_exchange_rate: StdDecimal::from_ratio(11_u128, 10_u128), // 1 stluna = 1.1 luna
            total_bond_bluna_amount: Default::default(),
            total_bond_stluna_amount: Default::default(),
            last_index_modification: 0,
            prev_hub_balance: Default::default(),
            last_unbonded_time: 0,
            last_processed_batch: 0,
            total_bond_amount: Default::default(),
            exchange_rate: Default::default(),
        });

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceByReference {
                    asset_reference: asset_reference.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        // stLuna/USD = stLuna/Luna * Luna/USD
        assert_eq!(price, Decimal::from_ratio(1034_u128, 10_u128));
    }

    // TEST_HELPERS
    fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner"),
        };
        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        deps
    }
}
