use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult, Uint128};
use mars_red_bank_types::red_bank::Debt;

use crate::{
    error::ContractError,
    state::{DEBTS, OWNER, UNCOLLATERALIZED_LOAN_LIMITS},
};

/// Update uncollateralized loan limit by a given amount in base asset
pub fn update_uncollateralized_loan_limit(
    deps: DepsMut,
    info: MessageInfo,
    user_addr: Addr,
    denom: String,
    new_limit: Uint128,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    // Check that the user has no collateralized debt
    let current_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (&user_addr, &denom))?
        .unwrap_or_else(Uint128::zero);
    let current_debt = DEBTS
        .may_load(deps.storage, (&user_addr, &denom))?
        .map(|debt| debt.amount_scaled)
        .unwrap_or_else(Uint128::zero);
    if current_limit.is_zero() && !current_debt.is_zero() {
        return Err(ContractError::UserHasCollateralizedDebt {});
    }
    if !current_limit.is_zero() && new_limit.is_zero() && !current_debt.is_zero() {
        return Err(ContractError::UserHasUncollateralizedDebt {});
    }

    UNCOLLATERALIZED_LOAN_LIMITS.save(deps.storage, (&user_addr, &denom), &new_limit)?;

    DEBTS.update(deps.storage, (&user_addr, &denom), |debt_opt: Option<Debt>| -> StdResult<_> {
        let mut debt = debt_opt.unwrap_or(Debt {
            amount_scaled: Uint128::zero(),
            uncollateralized: false,
        });
        // if limit == 0 then uncollateralized = false, otherwise uncollateralized = true
        debt.uncollateralized = !new_limit.is_zero();
        Ok(debt)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_uncollateralized_loan_limit")
        .add_attribute("user", user_addr)
        .add_attribute("denom", denom)
        .add_attribute("new_allowance", new_limit))
}
