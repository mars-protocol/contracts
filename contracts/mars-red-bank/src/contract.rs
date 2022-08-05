#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use mars_outpost::red_bank::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::helpers::get_denom_amount_from_coins;
use crate::{execute, queries};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    execute::initialize(deps, msg.config)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateConfig {
            config,
        } => execute::update_config(deps, env, info, config),
        ExecuteMsg::InitAsset {
            denom,
            asset_params,
            asset_symbol,
        } => execute::init_asset(deps, env, info, denom, asset_params, asset_symbol),
        ExecuteMsg::InitAssetTokenCallback {
            denom,
        } => execute::init_asset_token_callback(deps, info, denom),
        ExecuteMsg::UpdateAsset {
            denom,
            asset_params,
        } => execute::update_asset(deps, env, info, denom, asset_params),
        ExecuteMsg::UpdateUncollateralizedLoanLimit {
            user_address,
            denom,
            new_limit,
        } => execute::update_uncollateralized_loan_limit(
            deps,
            info,
            api.addr_validate(&user_address)?,
            denom,
            new_limit,
        ),
        ExecuteMsg::Deposit {
            denom,
            on_behalf_of,
        } => execute::deposit(
            deps,
            env,
            info.sender,
            on_behalf_of,
            denom.clone(),
            get_denom_amount_from_coins(&info.funds, &denom)?,
        ),
        ExecuteMsg::Withdraw {
            denom,
            amount,
            recipient,
        } => execute::withdraw(deps, env, info, denom, amount, recipient),

        ExecuteMsg::Borrow {
            denom,
            amount,
            recipient,
        } => execute::borrow(deps, env, info, denom, amount, recipient),

        ExecuteMsg::Repay {
            denom,
            on_behalf_of,
        } => execute::repay(
            deps,
            env,
            info.sender,
            on_behalf_of,
            denom.clone(),
            get_denom_amount_from_coins(&info.funds, &denom)?,
        ),
        ExecuteMsg::Liquidate {
            collateral_denom,
            debt_denom,
            user_address,
            receive_ma_token,
        } => execute::liquidate(
            deps,
            env,
            info.sender,
            collateral_denom,
            debt_denom.clone(),
            api.addr_validate(&user_address)?,
            get_denom_amount_from_coins(&info.funds, &debt_denom)?,
            receive_ma_token,
        ),
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom,
            enable,
        } => execute::update_asset_collateral_status(deps, env, info, denom, enable),
        ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address,
            recipient_address,
            sender_previous_balance,
            recipient_previous_balance,
            amount,
        } => execute::finalize_liquidity_token_transfer(
            deps,
            env,
            info,
            sender_address,
            recipient_address,
            sender_previous_balance,
            recipient_previous_balance,
            amount,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::Market {
            denom,
        } => to_binary(&queries::market(deps, denom)?),
        QueryMsg::Markets {
            start_after,
            limit,
        } => to_binary(&queries::markets(deps, start_after, limit)?),
        QueryMsg::UserDebt {
            user_address,
        } => {
            to_binary(&queries::user_debt(deps, env, deps.api.addr_validate(&user_address)?)?)
        }
        QueryMsg::UserAssetDebt {
            user_address,
            denom,
        } => to_binary(&queries::user_asset_debt(
            deps,
            env,
            deps.api.addr_validate(&user_address)?,
            denom,
        )?),
        QueryMsg::UserCollateral {
            user_address,
        } => to_binary(&queries::user_collateral(
            deps,
            deps.api.addr_validate(&user_address)?,
        )?),
        QueryMsg::UncollateralizedLoanLimit {
            user_address,
            denom,
        } => to_binary(&queries::uncollateralized_loan_limit(
            deps,
            deps.api.addr_validate(&user_address)?,
            denom,
        )?),
        QueryMsg::ScaledLiquidityAmount {
            denom,
            amount,
        } => to_binary(&queries::scaled_liquidity_amount(deps, env, denom, amount)?),
        QueryMsg::ScaledDebtAmount {
            denom,
            amount,
        } => to_binary(&queries::scaled_debt_amount(deps, env, denom, amount)?),
        QueryMsg::UnderlyingLiquidityAmount {
            ma_token_address,
            amount_scaled,
        } => to_binary(&queries::underlying_liquidity_amount(
            deps,
            env,
            ma_token_address,
            amount_scaled,
        )?),
        QueryMsg::UnderlyingDebtAmount {
            denom,
            amount_scaled,
        } => to_binary(&queries::underlying_debt_amount(deps, env, denom, amount_scaled)?),
        QueryMsg::UserPosition {
            user_address,
        } => to_binary(&queries::user_position(
            deps,
            env,
            deps.api.addr_validate(&user_address)?,
        )?),
    }
}
