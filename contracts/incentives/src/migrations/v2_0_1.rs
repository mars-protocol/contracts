use cosmwasm_std::{Addr, DepsMut, Empty, Env, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{ASTRO_USER_LP_DEPOSITS, USER_ASTRO_INCENTIVE_STATES},
};

const FROM_VERSION: &str = "2.0.0";

pub fn migrate(mut deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    clear_zero_balances(&mut deps)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn clear_zero_balances(deps: &mut DepsMut) -> Result<(), ContractError> {
    
    // Collect all LP positions that are zero
    let zero_balance_items= ASTRO_USER_LP_DEPOSITS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok(((user_id, account), value)) if value.is_zero() => {
                    Some(Ok(((user_id.to_string(), account.to_string()), value)))
                }
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Iterate all LP positions that are zero, and delete the incentive indexes
    for ((account_id, denom), _) in zero_balance_items.iter() {
        ASTRO_USER_LP_DEPOSITS.remove(deps.storage, (account_id, denom));
        
        // Get all incentives for (user, lp_token_denom) key
        let prefix = USER_ASTRO_INCENTIVE_STATES.prefix((account_id, denom));

        // Iterate over all reward_denom keys
        let keys_to_remove =
            prefix.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<String>>>()?;

        // Delete each matching (account_id, lp_token_denom, reward_denom) incentive index.
        for incentive_denom in keys_to_remove {
            USER_ASTRO_INCENTIVE_STATES
                .remove(deps.storage, (account_id, denom.as_str(), &incentive_denom));
        }
    }

    Ok(())
}