use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    error::MarsError,
    red_bank::{InitOrUpdateAssetParams, Market},
};
use mars_utils::helpers::validate_native_denom;

use crate::{
    error::ContractError,
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, MARKETS, OWNER},
};

/// Initialize asset if not exist.
/// Initialization requires that all params are provided and there is no asset in state.
pub fn init_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    validate_native_denom(&denom)?;

    if MARKETS.may_load(deps.storage, &denom)?.is_some() {
        return Err(ContractError::AssetAlreadyInitialized {});
    }

    let new_market = create_market(env.block.time.seconds(), &denom, params)?;
    MARKETS.save(deps.storage, &denom, &new_market)?;

    Ok(Response::new().add_attribute("action", "init_asset").add_attribute("denom", denom))
}

/// Initialize new market
pub fn create_market(
    block_time: u64,
    denom: &str,
    params: InitOrUpdateAssetParams,
) -> Result<Market, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let InitOrUpdateAssetParams {
        reserve_factor,
        interest_rate_model,
    } = params;

    // All fields should be available
    let available = reserve_factor.is_some() && interest_rate_model.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    }

    let new_market = Market {
        denom: denom.to_string(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        interest_rate_model: interest_rate_model.unwrap(),
    };

    new_market.validate()?;

    Ok(new_market)
}

/// Update asset with new params.
pub fn update_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let market_option = MARKETS.may_load(deps.storage, &denom)?;
    match market_option {
        None => Err(ContractError::AssetNotInitialized {}),
        Some(mut market) => {
            // Destructuring a struct’s fields into separate variables in order to force
            // compile error if we add more params
            let InitOrUpdateAssetParams {
                reserve_factor,
                interest_rate_model,
            } = params;

            // If reserve factor or interest rates are updated we update indexes with
            // current values before applying the change to prevent applying this
            // new params to a period where they were not valid yet. Interests rates are
            // recalculated after changes are applied.
            let should_update_interest_rates = (reserve_factor.is_some()
                && reserve_factor.unwrap() != market.reserve_factor)
                || interest_rate_model.is_some();

            let mut response = Response::new();

            if should_update_interest_rates {
                let config = CONFIG.load(deps.storage)?;
                let addresses = address_provider::helpers::query_contract_addrs(
                    deps.as_ref(),
                    &config.address_provider,
                    vec![MarsAddressType::Incentives, MarsAddressType::RewardsCollector],
                )?;
                let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
                let incentives_addr = &addresses[&MarsAddressType::Incentives];

                response = apply_accumulated_interests(
                    deps.storage,
                    &env,
                    &mut market,
                    rewards_collector_addr,
                    incentives_addr,
                    response,
                )?;
            }

            let mut updated_market = Market {
                reserve_factor: reserve_factor.unwrap_or(market.reserve_factor),
                interest_rate_model: interest_rate_model.unwrap_or(market.interest_rate_model),
                ..market
            };

            updated_market.validate()?;

            if should_update_interest_rates {
                response = update_interest_rates(&env, &mut updated_market, response)?;
            }
            MARKETS.save(deps.storage, &denom, &updated_market)?;

            Ok(response.add_attribute("action", "update_asset").add_attribute("denom", denom))
        }
    }
}
