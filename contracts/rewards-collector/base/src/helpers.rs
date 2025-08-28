use cosmwasm_std::{Addr, Deps, QuerierWrapper, Uint128};
use mars_owner::Owner;
use mars_types::rewards_collector::Config;

use crate::{ContractError, ContractResult};

/// For a denom with an optional Uint128 amount,
/// - if the amount is provided, assert that it is no larger than the available balance;
/// - if not provided, use the available balance as default.
pub(crate) fn unwrap_option_amount(
    querier: &QuerierWrapper<impl cosmwasm_std::CustomQuery>,
    addr: &Addr,
    denom: &str,
    amount: Option<Uint128>,
) -> ContractResult<Uint128> {
    let balance = querier.query_balance(addr, denom)?.amount;
    if let Some(amount) = amount {
        if amount > balance {
            return Err(ContractError::AmountToDistributeTooLarge {
                amount,
                balance,
            });
        }
        Ok(amount)
    } else {
        Ok(balance)
    }
}

pub(crate) fn ensure_distributor_whitelisted(
    deps: Deps,
    cfg: &Config,
    owner: &Owner,
    sender: &Addr,
) -> ContractResult<()> {
    // Owner can always distribute rewards
    if owner.is_owner(deps.storage, sender)? {
        return Ok(());
    }

    if cfg.whitelisted_distributors.is_empty() || !cfg.whitelisted_distributors.contains(sender) {
        return Err(ContractError::UnauthorizedDistributor {
            sender: sender.to_string(),
        });
    }

    Ok(())
}

/// Convert an optional Uint128 amount to string. If the amount is undefined, return `undefined`
pub(crate) fn stringify_option_amount(amount: Option<Uint128>) -> String {
    amount.map_or_else(|| "undefined".to_string(), |amount| amount.to_string())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies_with_balance, MOCK_CONTRACT_ADDR},
    };

    use super::*;

    #[test]
    fn unwrapping_option_amount() {
        let deps = mock_dependencies_with_balance(&[
            coin(88888, "uatom"),
            coin(1234, "uusdc"),
            coin(8964, "umars"),
        ]);

        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                None
            ),
            Ok(Uint128::new(88888))
        );
        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                Some(Uint128::new(12345))
            ),
            Ok(Uint128::new(12345))
        );
        assert_eq!(
            unwrap_option_amount(
                &deps.as_ref().querier,
                &Addr::unchecked(MOCK_CONTRACT_ADDR),
                "uatom",
                Some(Uint128::new(99999))
            ),
            Err(ContractError::AmountToDistributeTooLarge {
                amount: Uint128::new(99999),
                balance: Uint128::new(88888),
            })
        );
    }

    #[test]
    fn stringifying_option_amount() {
        assert_eq!(stringify_option_amount(Some(Uint128::new(42069))), "42069".to_string());
        assert_eq!(stringify_option_amount(None), "undefined".to_string());
    }
}
