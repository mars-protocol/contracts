use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use mars_red_bank_types::red_bank::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::{error::ContractError, execute, query};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    execute::instantiate(deps, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => execute::update_owner(deps, info, update),
        ExecuteMsg::UpdateConfig {
            config,
        } => execute::update_config(deps, info, config),
        ExecuteMsg::InitAsset {
            denom,
            params,
        } => execute::init_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateAsset {
            denom,
            params,
        } => execute::update_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateUncollateralizedLoanLimit {
            user,
            denom,
            new_limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            execute::update_uncollateralized_loan_limit(deps, info, user_addr, denom, new_limit)
        }
        ExecuteMsg::Deposit {
            on_behalf_of,
        } => {
            let sent_coin = cw_utils::one_coin(&info)?;
            execute::deposit(deps, env, info, on_behalf_of, sent_coin.denom, sent_coin.amount)
        }
        ExecuteMsg::Withdraw {
            denom,
            amount,
            recipient,
        } => {
            cw_utils::nonpayable(&info)?;
            execute::withdraw(deps, env, info, denom, amount, recipient)
        },
        ExecuteMsg::Borrow {
            denom,
            amount,
            recipient,
        } => {
            cw_utils::nonpayable(&info)?;
            execute::borrow(deps, env, info, denom, amount, recipient)
        },
        ExecuteMsg::Repay {
            on_behalf_of,
        } => {
            let sent_coin = cw_utils::one_coin(&info)?;
            execute::repay(deps, env, info, on_behalf_of, sent_coin.denom, sent_coin.amount)
        }
        ExecuteMsg::Liquidate {
            user,
            collateral_denom,
            recipient,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            let sent_coin = cw_utils::one_coin(&info)?;
            execute::liquidate(
                deps,
                env,
                info,
                collateral_denom,
                sent_coin.denom,
                user_addr,
                sent_coin.amount,
                recipient,
            )
        }
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom,
            enable,
        } => {
            cw_utils::nonpayable(&info)?;
            execute::update_asset_collateral_status(deps, env, info, denom, enable)
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query::query_config(deps)?),
        QueryMsg::Market {
            denom,
        } => to_binary(&query::query_market(deps, denom)?),
        QueryMsg::Markets {
            start_after,
            limit,
        } => to_binary(&query::query_markets(deps, start_after, limit)?),
        QueryMsg::UncollateralizedLoanLimit {
            user,
            denom,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_uncollateralized_loan_limit(deps, user_addr, denom)?)
        }
        QueryMsg::UncollateralizedLoanLimits {
            user,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_uncollateralized_loan_limits(
                deps,
                user_addr,
                start_after,
                limit,
            )?)
        }
        QueryMsg::UserDebt {
            user,
            denom,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_user_debt(deps, &env.block, user_addr, denom)?)
        }
        QueryMsg::UserDebts {
            user,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_user_debts(deps, &env.block, user_addr, start_after, limit)?)
        }
        QueryMsg::UserCollateral {
            user,
            denom,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_user_collateral(deps, &env.block, user_addr, denom)?)
        }
        QueryMsg::UserCollaterals {
            user,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_user_collaterals(
                deps,
                &env.block,
                user_addr,
                start_after,
                limit,
            )?)
        }
        QueryMsg::UserPosition {
            user,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_binary(&query::query_user_position(deps, env, user_addr)?)
        }
        QueryMsg::ScaledLiquidityAmount {
            denom,
            amount,
        } => to_binary(&query::query_scaled_liquidity_amount(deps, env, denom, amount)?),
        QueryMsg::ScaledDebtAmount {
            denom,
            amount,
        } => to_binary(&query::query_scaled_debt_amount(deps, env, denom, amount)?),
        QueryMsg::UnderlyingLiquidityAmount {
            denom,
            amount_scaled,
        } => to_binary(&query::query_underlying_liquidity_amount(deps, env, denom, amount_scaled)?),
        QueryMsg::UnderlyingDebtAmount {
            denom,
            amount_scaled,
        } => to_binary(&query::query_underlying_debt_amount(deps, env, denom, amount_scaled)?),
    };
    res.map_err(Into::into)
}
