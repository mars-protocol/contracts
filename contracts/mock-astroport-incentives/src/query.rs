use std::str::FromStr;

use astroport_v5::asset::Asset;
use cosmwasm_std::{Coin, Decimal256, Deps, Env, StdResult, Uint128};

use crate::state::{ASTRO_LP_INCENTIVE_DEPOSITS, INCENTIVE_SCHEDULES};

pub fn query_rewards(
    deps: Deps,
    _: Env,
    sender: String,
    lp_token: String,
) -> StdResult<Vec<Asset>> {
    let deposits = ASTRO_LP_INCENTIVE_DEPOSITS.may_load(deps.storage, (&sender, &lp_token))?;
    match deposits {
        Some(_) => Ok(INCENTIVE_SCHEDULES
            .prefix(&lp_token)
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(
                |item: Result<
                    (String, astroport_v5::incentives::IncentivesSchedule),
                    cosmwasm_std::StdError,
                >| {
                    let (reward_denom, schedule) = item.unwrap();
                    // Note - this gives all rewards to the claimer, but in reality we would need to calculate the rewards for each user.
                    let amount =
                        schedule.rps.checked_mul(Decimal256::from_str("5000").unwrap()).unwrap();
                    Coin {
                        amount: Uint128::try_from(amount.to_uint_floor()).unwrap(),
                        denom: reward_denom,
                    }
                    .into()
                },
            )
            .collect()),

        None => Err(cosmwasm_std::StdError::NotFound {
            kind: "position not found".to_string(),
        }),
    }
}

pub fn query_deposit(deps: Deps, user: String, lp_token: String) -> StdResult<Uint128> {
    Ok(ASTRO_LP_INCENTIVE_DEPOSITS.may_load(deps.storage, (&user, &lp_token))?.unwrap_or_default())
}
