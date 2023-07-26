use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use mars_interest_rate::get_scaled_liquidity_amount;
use mars_red_bank_types::address_provider::{self, MarsAddressType};

use crate::{
    error::ContractError,
    helpers::{query_asset_params, query_total_deposit},
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, MARKETS},
    user::User,
};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut market = MARKETS.load(deps.storage, &denom)?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Params,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let params_addr = &addresses[&MarsAddressType::Params];

    let asset_params = query_asset_params(&deps.querier, params_addr, &denom)?;

    if !asset_params.red_bank.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            denom,
        });
    }

    let total_deposits = query_total_deposit(&deps.querier, params_addr, &denom)?;
    if total_deposits.amount.checked_add(deposit_amount)? > asset_params.red_bank.deposit_cap {
        return Err(ContractError::DepositCapExceeded {
            denom,
        });
    }

    let mut response = Response::new();

    // update indexes and interest rates
    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }
    let deposit_amount_scaled =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;

    response = User(&info.sender).increase_collateral(
        deps.storage,
        &market,
        deposit_amount_scaled,
        incentives_addr,
        response,
    )?;

    market.increase_collateral(deposit_amount_scaled)?;

    response = update_interest_rates(&env, &mut market, response)?;

    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(response
        .add_attribute("action", "deposit")
        .add_attribute("sender", &info.sender)
        .add_attribute("denom", denom)
        .add_attribute("amount", deposit_amount)
        .add_attribute("amount_scaled", deposit_amount_scaled))
}
