use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use mars_interest_rate::{
    get_scaled_debt_amount, get_underlying_debt_amount, get_underlying_liquidity_amount,
};
use mars_types::{address_provider, address_provider::MarsAddressType};
use mars_utils::helpers::build_send_asset_msg;

use crate::{
    error::ContractError,
    health::assert_below_max_ltv_after_borrow,
    helpers::query_asset_params,
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, MARKETS},
    user::User,
};

/// Add debt for the borrower and send the borrowed funds
pub fn borrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    borrow_amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let borrower = User(&info.sender);

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

    let asset_params = query_asset_params(&deps.querier, params_addr, &denom)?;

    if !asset_params.red_bank.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            denom,
        });
    }

    // Load market and user state
    let mut borrow_market = MARKETS.load(deps.storage, &denom)?;

    let collateral_balance_before = get_underlying_liquidity_amount(
        borrow_market.collateral_total_scaled,
        &borrow_market,
        env.block.time.seconds(),
    )?;
    let debt_balance_before = get_underlying_debt_amount(
        borrow_market.debt_total_scaled,
        &borrow_market,
        env.block.time.seconds(),
    )?;

    // Cannot borrow zero amount or more than available liquidity
    let available_liquidity = collateral_balance_before.checked_sub(debt_balance_before)?;
    if borrow_amount.is_zero() || borrow_amount > available_liquidity {
        return Err(ContractError::InvalidBorrowAmount {
            denom,
        });
    }

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if info.sender != credit_manager_addr {
        if !assert_below_max_ltv_after_borrow(
            &deps.as_ref(),
            &env,
            borrower.address(),
            "",
            oracle_addr,
            params_addr,
            &denom,
            borrow_amount,
        )? {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    } else {
        uncollateralized_debt = true;
    }

    let mut response = Response::new();

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut borrow_market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    // Set new debt
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;

    borrow_market.increase_debt(borrow_amount_scaled)?;
    borrower.increase_debt(deps.storage, &denom, borrow_amount_scaled, uncollateralized_debt)?;

    response = update_interest_rates(&env, &mut borrow_market, response)?;
    MARKETS.save(deps.storage, &denom, &borrow_market)?;

    // Send borrow amount to borrower or another recipient
    let recipient_addr = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
    } else {
        borrower.address().clone()
    };

    Ok(response
        .add_message(build_send_asset_msg(&recipient_addr, &denom, borrow_amount))
        .add_attribute("action", "borrow")
        .add_attribute("sender", borrower)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("denom", denom)
        .add_attribute("amount", borrow_amount)
        .add_attribute("amount_scaled", borrow_amount_scaled))
}
