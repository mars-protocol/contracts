use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, Uint128};
use mars_interest_rate::{
    get_scaled_debt_amount, get_scaled_liquidity_amount, get_underlying_debt_amount,
    get_underlying_liquidity_amount,
};
use mars_liquidation::liquidation::calculate_liquidation_amounts;
use mars_types::{
    address_provider::{self, MarsAddressType},
    keys::{UserId, UserIdKey},
};
use mars_utils::helpers::{build_send_asset_msg, option_string_to_addr};

use crate::{
    error::ContractError,
    health::get_health_and_positions,
    helpers::{query_asset_params, query_target_health_factor},
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS},
    user::User,
};

/// Execute loan liquidations on under-collateralized loans
pub fn liquidate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_denom: String,
    debt_denom: String,
    liquidatee_addr: Addr,
    sent_debt_amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();

    let liquidatee = User(&liquidatee_addr);

    // The recipient address for receiving collateral
    let recipient_addr = option_string_to_addr(deps.api, recipient, info.sender.clone())?;
    let recipient = User(&recipient_addr);

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Params,
            MarsAddressType::CreditManager,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let params_addr = &addresses[&MarsAddressType::Params];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    // 1. Validate liquidation

    // User cannot liquidate themselves
    if info.sender == liquidatee_addr {
        return Err(ContractError::CannotLiquidateSelf {});
    }

    // Cannot liquidate credit manager users. They have own liquidation logic in credit-manager contract.
    if liquidatee_addr == credit_manager_addr {
        return Err(ContractError::CannotLiquidateCreditManager {});
    };

    let user_id = UserId::credit_manager(liquidatee_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into()?;

    // check if the user has enabled the collateral asset as collateral
    let user_collateral = COLLATERALS
        .may_load(deps.storage, (&user_id_key, &collateral_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoCollateralBalance {})?;
    if !user_collateral.enabled {
        return Err(ContractError::CannotLiquidateWhenCollateralUnset {
            denom: collateral_denom,
        });
    }

    // check if user has outstanding debt in the deposited asset that needs to be repayed
    let user_debt = DEBTS
        .may_load(deps.storage, (&liquidatee_addr, &debt_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoDebtBalance {})?;

    // check if user has available collateral in specified collateral asset to be liquidated
    let collateral_market = MARKETS.load(deps.storage, &collateral_denom)?;

    // 2. Compute health factor
    let (health, assets_positions) = get_health_and_positions(
        &deps.as_ref(),
        &env,
        &liquidatee_addr,
        "",
        oracle_addr,
        params_addr,
        true,
    )?;

    if !health.is_liquidatable() {
        return Err(ContractError::CannotLiquidateHealthyPosition {});
    }

    let debt_market = if debt_denom != collateral_denom {
        MARKETS.load(deps.storage, &debt_denom)?
    } else {
        collateral_market.clone()
    };

    // 3. Compute debt to repay and collateral to liquidate
    let collateral_price = assets_positions
        .get(&collateral_denom)
        .ok_or(ContractError::CannotLiquidateWhenNoCollateralBalance {})?
        .asset_price;
    let debt_price = assets_positions
        .get(&debt_denom)
        .ok_or(ContractError::CannotLiquidateWhenNoDebtBalance {})?
        .asset_price;

    let mut response = Response::new();

    let user_debt_amount =
        get_underlying_debt_amount(user_debt.amount_scaled, &debt_market, block_time)?;

    let collateral_params = query_asset_params(&deps.querier, params_addr, &collateral_denom)?;
    let target_health_factor = query_target_health_factor(&deps.querier, params_addr)?;

    let user_collateral_amount = get_underlying_liquidity_amount(
        user_collateral.amount_scaled,
        &collateral_market,
        block_time,
    )?;
    let (
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_received_by_liquidator,
    ) = calculate_liquidation_amounts(
        user_collateral_amount,
        collateral_price,
        &collateral_params,
        user_debt_amount,
        sent_debt_amount,
        debt_price,
        target_health_factor,
        &health,
    )?;
    let protocol_fee = collateral_amount_to_liquidate - collateral_amount_received_by_liquidator;

    let refund_amount = sent_debt_amount - debt_amount_to_repay;

    let collateral_amount_to_liquidate_scaled = get_scaled_liquidity_amount(
        collateral_amount_to_liquidate,
        &collateral_market,
        block_time,
    )?;

    let collateral_amount_received_by_liquidator_scaled = get_scaled_liquidity_amount(
        collateral_amount_received_by_liquidator,
        &collateral_market,
        block_time,
    )?;

    let protocol_fee_scaled =
        get_scaled_liquidity_amount(protocol_fee, &collateral_market, block_time)?;

    // 4. Transfer collateral shares from the user to the liquidator and rewards-collector (protocol fee)
    response = liquidatee.decrease_collateral(
        deps.storage,
        &collateral_market,
        collateral_amount_to_liquidate_scaled,
        incentives_addr,
        response,
        None,
    )?;
    response = recipient.increase_collateral(
        deps.storage,
        &collateral_market,
        collateral_amount_received_by_liquidator_scaled,
        incentives_addr,
        response,
        None,
    )?;
    if !protocol_fee.is_zero() {
        response = User(rewards_collector_addr).increase_collateral(
            deps.storage,
            &collateral_market,
            protocol_fee_scaled,
            incentives_addr,
            response,
            None,
        )?;
    }

    // 5. Reduce the user's debt shares
    let user_debt_amount_after = user_debt_amount.checked_sub(debt_amount_to_repay)?;
    let user_debt_amount_scaled_after =
        get_scaled_debt_amount(user_debt_amount_after, &debt_market, block_time)?;

    // Compute delta so it can be substracted to total debt
    let debt_amount_scaled_delta =
        user_debt.amount_scaled.checked_sub(user_debt_amount_scaled_after)?;

    liquidatee.decrease_debt(deps.storage, &debt_denom, debt_amount_scaled_delta)?;

    let market_debt_total_scaled_after =
        debt_market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

    // 6. Update markets
    let mut debt_market_after = debt_market;
    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut debt_market_after,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;
    debt_market_after.debt_total_scaled = market_debt_total_scaled_after;
    response = update_interest_rates(&env, &mut debt_market_after, response)?;
    MARKETS.save(deps.storage, &debt_denom, &debt_market_after)?;

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    if !refund_amount.is_zero() {
        response =
            response.add_message(build_send_asset_msg(&info.sender, &debt_denom, refund_amount));
    }

    Ok(response
        .add_attribute("action", "liquidate")
        .add_attribute("user", liquidatee)
        .add_attribute("liquidator", info.sender.to_string())
        .add_attribute("recipient", recipient)
        .add_attribute("collateral_denom", collateral_denom)
        .add_attribute("collateral_amount", collateral_amount_to_liquidate)
        .add_attribute("collateral_amount_scaled", collateral_amount_to_liquidate_scaled)
        .add_attribute("debt_denom", debt_denom)
        .add_attribute("debt_amount", debt_amount_to_repay)
        .add_attribute("debt_amount_scaled", debt_amount_scaled_delta))
}
