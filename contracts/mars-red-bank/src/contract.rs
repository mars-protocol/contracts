use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw_utils::{must_pay, one_coin};

use mars_outpost::red_bank::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::{execute, query};

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
        ExecuteMsg::UpdateConfig {
            config,
        } => execute::update_config(deps, env, info, config),
        ExecuteMsg::InitAsset {
            denom,
            params,
        } => execute::init_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateAsset {
            denom,
            params,
        } => execute::update_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateUncollateralizedLoanLimit {
            user_address,
            denom,
            new_limit,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            execute::update_uncollateralized_loan_limit(
                deps, env, info, user_addr, denom, new_limit,
            )
        }
        ExecuteMsg::Deposit {
            denom,
            on_behalf_of,
        } => {
            let deposit_amount = must_pay(&info, &denom)?;
            let depositor_address = info.sender.clone();
            execute::deposit(
                deps,
                env,
                info,
                depositor_address,
                on_behalf_of,
                denom,
                deposit_amount,
            )
        }
        ExecuteMsg::Withdraw {
            denom,
            amount,
            recipient: recipient_address,
        } => execute::withdraw(deps, env, info, denom, amount, recipient_address),
        ExecuteMsg::Borrow {
            denom,
            amount,
            recipient: recipient_address,
        } => execute::borrow(deps, env, info, denom, amount, recipient_address),
        ExecuteMsg::Repay {
            denom,
            on_behalf_of,
        } => {
            let repayer_address = info.sender.clone();
            let repay_amount = must_pay(&info, &denom)?;
            execute::repay(deps, env, info, repayer_address, on_behalf_of, denom, repay_amount)
        }
        ExecuteMsg::Liquidate {
            collateral_denom,
            user_address,
        } => {
            let sender = info.sender.clone();
            let user_addr = deps.api.addr_validate(&user_address)?;
            let sent_debt_asset = one_coin(&info)?;
            execute::liquidate(
                deps,
                env,
                info,
                sender,
                collateral_denom,
                sent_debt_asset.denom,
                user_addr,
                sent_debt_asset.amount,
            )
        }
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom,
            enable,
        } => execute::update_asset_collateral_status(deps, env, info, denom, enable),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query::query_config(deps)?),
        QueryMsg::Market {
            denom,
        } => to_binary(&query::query_market(deps, denom)?),
        QueryMsg::Markets {
            start_after,
            limit,
        } => to_binary(&query::query_markets(deps, start_after, limit)?),
        QueryMsg::UserDebt {
            user_address,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_user_debt(deps, env, address)?)
        }
        QueryMsg::UserAssetDebt {
            user_address,
            denom,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_user_asset_debt(deps, env, address, denom)?)
        }
        QueryMsg::UserCollateral {
            user_address,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_user_collateral(deps, env, address)?)
        }
        QueryMsg::UserAssetCollateral {
            user_address,
            denom,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_user_asset_collateral(deps, env, address, denom)?)
        }
        QueryMsg::UncollateralizedLoanLimit {
            user_address,
            denom,
        } => {
            let user_address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_uncollateralized_loan_limit(deps, user_address, denom)?)
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
        QueryMsg::UserPosition {
            user_address,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query::query_user_position(deps, env, address)?)
        }
    }
}
