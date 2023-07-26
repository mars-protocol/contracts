use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use mars_red_bank_types::{address_provider, address_provider::MarsAddressType};
use mars_utils::helpers::build_send_asset_msg;

use crate::{
    error::ContractError,
    health::assert_below_liq_threshold_after_withdraw,
    interest_rates::{
        apply_accumulated_interests, get_scaled_liquidity_amount, get_underlying_liquidity_amount,
        update_interest_rates,
    },
    state::{CONFIG, MARKETS},
    user::User,
};

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    amount: Option<Uint128>,
    recipient: Option<String>,
    account_id: Option<String>,
) -> Result<Response, ContractError> {
    let withdrawer = User(&info.sender);

    let mut market = MARKETS.load(deps.storage, &denom)?;

    let collateral = withdrawer.collateral(deps.storage, &denom, account_id.clone())?;
    let withdrawer_balance_scaled_before = collateral.amount_scaled;

    if withdrawer_balance_scaled_before.is_zero() {
        return Err(ContractError::UserNoCollateralBalance {
            user: withdrawer.into(),
            denom,
        });
    }

    let withdrawer_balance_before = get_underlying_liquidity_amount(
        withdrawer_balance_scaled_before,
        &market,
        env.block.time.seconds(),
    )?;

    let withdraw_amount = match amount {
        // Check user has sufficient balance to send back
        Some(amount) if amount.is_zero() || amount > withdrawer_balance_before => {
            return Err(ContractError::InvalidWithdrawAmount {
                denom,
            });
        }
        Some(amount) => amount,
        // If no amount is specified, the full balance is withdrawn
        None => withdrawer_balance_before,
    };

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Params,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let params_addr = &addresses[&MarsAddressType::Params];

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if collateral.enabled
        && withdrawer.is_borrowing(deps.storage)
        && !assert_below_liq_threshold_after_withdraw(
            &deps.as_ref(),
            &env,
            withdrawer.address(),
            oracle_addr,
            params_addr,
            &denom,
            withdraw_amount,
        )?
    {
        return Err(ContractError::InvalidHealthFactorAfterWithdraw {});
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

    // reduce the withdrawer's scaled collateral amount
    let withdrawer_balance_after = withdrawer_balance_before.checked_sub(withdraw_amount)?;
    let withdrawer_balance_scaled_after =
        get_scaled_liquidity_amount(withdrawer_balance_after, &market, env.block.time.seconds())?;

    let withdraw_amount_scaled =
        withdrawer_balance_scaled_before.checked_sub(withdrawer_balance_scaled_after)?;

    response = withdrawer.decrease_collateral(
        deps.storage,
        &market,
        withdraw_amount_scaled,
        incentives_addr,
        response,
        account_id,
    )?;

    market.decrease_collateral(withdraw_amount_scaled)?;

    response = update_interest_rates(&env, &mut market, response)?;

    MARKETS.save(deps.storage, &denom, &market)?;

    // send underlying asset to user or another recipient
    let recipient_addr = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
    } else {
        withdrawer.address().clone()
    };

    Ok(response
        .add_message(build_send_asset_msg(&recipient_addr, &denom, withdraw_amount))
        .add_attribute("action", "withdraw")
        .add_attribute("sender", withdrawer)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("denom", denom)
        .add_attribute("amount", withdraw_amount)
        .add_attribute("amount_scaled", withdraw_amount_scaled))
}
