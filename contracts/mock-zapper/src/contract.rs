#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{coin, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::error::ContractResult;
use crate::execute::{provide_liquidity, withdraw_liquidity};
use crate::query::{estimate_provide_liquidity, estimate_withdraw_liquidity};
use crate::state::{COIN_BALANCES, ORACLE};
use mars_rover::msg::zapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

pub const STARTING_LP_POOL_TOKENS: Uint128 = Uint128::new(1_000_000);

/// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
/// this contract expects to be pre-funded with LP tokens and it will simulate the mint.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    for config in msg.lp_configs {
        COIN_BALANCES.save(
            deps.storage,
            &config.lp_token_denom,
            &(
                coin(0, config.lp_pair_denoms.0),
                coin(0, config.lp_pair_denoms.1),
            ),
        )?;
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::ProvideLiquidity {
            lp_token_out,
            minimum_receive,
            ..
        } => provide_liquidity(deps, info, lp_token_out, minimum_receive),
        ExecuteMsg::WithdrawLiquidity { .. } => withdraw_liquidity(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::EstimateProvideLiquidity {
            lp_token_out,
            coins_in,
        } => to_binary(&estimate_provide_liquidity(&deps, &lp_token_out, coins_in)?),
        QueryMsg::EstimateWithdrawLiquidity { coin_in } => {
            to_binary(&estimate_withdraw_liquidity(deps.storage, &coin_in)?)
        }
    };
    res.map_err(Into::into)
}
