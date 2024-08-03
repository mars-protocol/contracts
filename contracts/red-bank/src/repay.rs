use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, Uint128};
use mars_interest_rate::{get_scaled_debt_amount, get_underlying_debt_amount};
use mars_types::address_provider::{self, MarsAddressType};
use mars_utils::helpers::build_send_asset_msg;

use crate::{
    error::ContractError,
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, DEBTS, MARKETS},
    user::User,
};

pub fn repay(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    on_behalf_of: Option<String>,
    denom: String,
    repay_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::CreditManager,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    let user_addr: Addr;
    let user = match on_behalf_of.as_ref() {
        // Cannot repay on behalf of credit-manager users. It creates accounting complexity for them.
        Some(address) if address == credit_manager_addr.as_str() => {
            return Err(ContractError::CannotRepayOnBehalfOfCreditManager {});
        }
        Some(address) => {
            user_addr = deps.api.addr_validate(address)?;
            User(&user_addr)
        }
        None => User(&info.sender),
    };

    // Check new debt
    let debt = DEBTS
        .may_load(deps.storage, (user.address(), &denom))?
        .ok_or(ContractError::CannotRepayZeroDebt {})?;

    let mut market = MARKETS.load(deps.storage, &denom)?;

    let mut response = Response::new();

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    let debt_amount_scaled_before = debt.amount_scaled;
    let debt_amount_before =
        get_underlying_debt_amount(debt.amount_scaled, &market, env.block.time.seconds())?;

    // If repay amount exceeds debt, refund any excess amounts
    let mut refund_amount = Uint128::zero();
    let mut debt_amount_after = Uint128::zero();
    if repay_amount > debt_amount_before {
        refund_amount = repay_amount - debt_amount_before;
        let refund_msg = build_send_asset_msg(&info.sender, &denom, refund_amount);
        response = response.add_message(refund_msg);
    } else {
        debt_amount_after = debt_amount_before - repay_amount;
    }

    let debt_amount_scaled_after =
        get_scaled_debt_amount(debt_amount_after, &market, env.block.time.seconds())?;

    let debt_amount_scaled_delta =
        debt_amount_scaled_before.checked_sub(debt_amount_scaled_after)?;

    market.decrease_debt(debt_amount_scaled_delta)?;
    user.decrease_debt(deps.storage, &denom, debt_amount_scaled_delta)?;

    response = update_interest_rates(&env, &mut market, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(response
        .add_attribute("action", "repay")
        .add_attribute("sender", &info.sender)
        .add_attribute("on_behalf_of", user)
        .add_attribute("denom", denom)
        .add_attribute("amount", repay_amount.checked_sub(refund_amount)?)
        .add_attribute("amount_scaled", debt_amount_scaled_delta))
}
