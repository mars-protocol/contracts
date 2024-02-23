use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use mars_types::red_bank::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::{
    asset, borrow, collateral, config, deposit, error::ContractError, instantiate, liquidate,
    migrations, query, repay, state::MIGRATION_GUARD, uncollateralized_loan, withdraw,
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    instantiate::instantiate(deps, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => config::update_owner(deps, info, update),
        ExecuteMsg::UpdateConfig {
            config,
        } => config::update_config(deps, info, config),
        ExecuteMsg::InitAsset {
            denom,
            params,
        } => asset::init_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateAsset {
            denom,
            params,
        } => asset::update_asset(deps, env, info, denom, params),
        ExecuteMsg::UpdateUncollateralizedLoanLimit {
            user,
            denom,
            new_limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            uncollateralized_loan::update_uncollateralized_loan_limit(
                deps, info, user_addr, denom, new_limit,
            )
        }
        ExecuteMsg::Deposit {
            account_id,
            on_behalf_of,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            let sent_coin = cw_utils::one_coin(&info)?;
            deposit::deposit(
                deps,
                env,
                info,
                on_behalf_of,
                sent_coin.denom,
                sent_coin.amount,
                account_id,
            )
        }
        ExecuteMsg::Withdraw {
            denom,
            amount,
            recipient,
            account_id,
            liquidation_related,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            cw_utils::nonpayable(&info)?;
            withdraw::withdraw(
                deps,
                env,
                info,
                denom,
                amount,
                recipient,
                account_id,
                liquidation_related.unwrap_or(false),
            )
        }
        ExecuteMsg::Borrow {
            denom,
            amount,
            recipient,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            cw_utils::nonpayable(&info)?;
            borrow::borrow(deps, env, info, denom, amount, recipient)
        }
        ExecuteMsg::Repay {
            on_behalf_of,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            let sent_coin = cw_utils::one_coin(&info)?;
            repay::repay(deps, env, info, on_behalf_of, sent_coin.denom, sent_coin.amount)
        }
        ExecuteMsg::Liquidate {
            user,
            collateral_denom,
            recipient,
        } => {
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            let user_addr = deps.api.addr_validate(&user)?;
            let sent_coin = cw_utils::one_coin(&info)?;
            liquidate::liquidate(
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
            MIGRATION_GUARD.assert_unlocked(deps.storage)?;
            cw_utils::nonpayable(&info)?;
            collateral::update_asset_collateral_status(deps, env, info, denom, enable)
        }
        ExecuteMsg::Migrate(msg) => {
            cw_utils::nonpayable(&info)?;
            migrations::v2_0_0::execute_migration(deps, info, msg)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_json_binary(&query::query_config(deps)?),
        QueryMsg::Market {
            denom,
        } => to_json_binary(&query::query_market(deps, denom)?),
        QueryMsg::Markets {
            start_after,
            limit,
        } => to_json_binary(&query::query_markets(deps, start_after, limit)?),
        QueryMsg::UncollateralizedLoanLimit {
            user,
            denom,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_uncollateralized_loan_limit(deps, user_addr, denom)?)
        }
        QueryMsg::UncollateralizedLoanLimits {
            user,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_uncollateralized_loan_limits(
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
            to_json_binary(&query::query_user_debt(deps, &env.block, user_addr, denom)?)
        }
        QueryMsg::UserDebts {
            user,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_debts(
                deps,
                &env.block,
                user_addr,
                start_after,
                limit,
            )?)
        }
        QueryMsg::UserCollateral {
            user,
            account_id,
            denom,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_collateral(
                deps, &env.block, user_addr, account_id, denom,
            )?)
        }
        QueryMsg::UserCollaterals {
            user,
            account_id,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_collaterals(
                deps,
                &env.block,
                user_addr,
                account_id,
                start_after,
                limit,
            )?)
        }
        QueryMsg::UserCollateralsV2 {
            user,
            account_id,
            start_after,
            limit,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_collaterals_v2(
                deps,
                &env.block,
                user_addr,
                account_id,
                start_after,
                limit,
            )?)
        }
        QueryMsg::UserPosition {
            user,
            account_id,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_position(deps, env, user_addr, account_id, false)?)
        }
        QueryMsg::UserPositionLiquidationPricing {
            user,
            account_id,
        } => {
            let user_addr = deps.api.addr_validate(&user)?;
            to_json_binary(&query::query_user_position(deps, env, user_addr, account_id, true)?)
        }
        QueryMsg::ScaledLiquidityAmount {
            denom,
            amount,
        } => to_json_binary(&query::query_scaled_liquidity_amount(deps, env, denom, amount)?),
        QueryMsg::ScaledDebtAmount {
            denom,
            amount,
        } => to_json_binary(&query::query_scaled_debt_amount(deps, env, denom, amount)?),
        QueryMsg::UnderlyingLiquidityAmount {
            denom,
            amount_scaled,
        } => to_json_binary(&query::query_underlying_liquidity_amount(
            deps,
            env,
            denom,
            amount_scaled,
        )?),
        QueryMsg::UnderlyingDebtAmount {
            denom,
            amount_scaled,
        } => to_json_binary(&query::query_underlying_debt_amount(deps, env, denom, amount_scaled)?),
    };
    res.map_err(Into::into)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::V1_0_0ToV2_0_0 {} => migrations::v2_0_0::migrate(deps),
        MigrateMsg::V2_0_0ToV2_0_1 {} => migrations::v2_0_1::migrate(deps),
    }
}
