use cosmwasm_std::{
    coin, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use cw_utils::one_coin;
use mars_red_bank_types::red_bank::{InitOrUpdateAssetParams, Market};

use crate::{
    helpers::{load_collateral_amount, load_debt_amount},
    state::{COLLATERAL_AMOUNT, COLLATERAL_DENOMS, DEBT_AMOUNT, MARKETS},
};

pub fn init_asset(
    deps: DepsMut,
    env: Env,
    denom: String,
    params: InitOrUpdateAssetParams,
) -> StdResult<Response> {
    // since this is just a mock, we don't do the same checks that we do in the
    // real red bank contract, such as sender == owner, validate denom, market
    // not already exists...
    let market = Market {
        denom: denom.clone(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        reserve_factor: params.reserve_factor.unwrap(),
        indexes_last_updated: env.block.time.seconds(),
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        interest_rate_model: params.interest_rate_model.unwrap(),
    };

    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(Response::new())
}

pub fn borrow(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    amount: Uint128,
) -> StdResult<Response> {
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), denom.clone()),
        &debt_amount.checked_add(amount)?.checked_add(Uint128::new(1))?, // The extra unit is simulated accrued interest
    )?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount.u128(), denom)],
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn repay(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let coin_sent =
        one_coin(&info).map_err(|_| StdError::generic_err("Repay coin reqs not met"))?;
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &coin_sent.denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender, coin_sent.denom.clone()),
        &debt_amount.checked_sub(coin_sent.amount)?,
    )?;

    Ok(Response::new())
}

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    account_id: Option<String>,
) -> StdResult<Response> {
    let to_deposit =
        one_coin(&info).map_err(|_| StdError::generic_err("Deposit coin reqs not met"))?;
    let collateral_amount = load_collateral_amount(
        deps.storage,
        info.sender.as_str(),
        &account_id.clone().unwrap_or_default(),
        &to_deposit.denom,
    )?;

    COLLATERAL_AMOUNT.save(
        deps.storage,
        (info.sender.to_string(), account_id.clone().unwrap_or_default(), to_deposit.denom.clone()),
        &collateral_amount.checked_add(to_deposit.amount)?.checked_add(Uint128::new(1))?, // The extra unit is simulated accrued yield
    )?;

    COLLATERAL_DENOMS.update(
        deps.storage,
        (info.sender.to_string(), account_id.unwrap_or_default()),
        |denoms_opt| -> StdResult<_> {
            let mut denoms = denoms_opt.unwrap_or_default();
            denoms.push(to_deposit.denom.clone());
            Ok(denoms)
        },
    )?;

    Ok(Response::new())
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
    amount: &Option<Uint128>,
    account_id: Option<String>,
) -> StdResult<Response> {
    let total_lent = load_collateral_amount(
        deps.storage,
        info.sender.as_str(),
        &account_id.clone().unwrap_or_default(),
        denom,
    )?;
    let amount_to_reclaim = amount.unwrap_or(total_lent);

    let new_amount = total_lent.checked_sub(amount_to_reclaim)?;
    COLLATERAL_AMOUNT.save(
        deps.storage,
        (info.sender.to_string(), account_id.clone().unwrap_or_default(), denom.to_string()),
        &new_amount,
    )?;

    COLLATERAL_DENOMS.update(
        deps.storage,
        (info.sender.to_string(), account_id.unwrap_or_default()),
        |denoms_opt| -> StdResult<_> {
            let mut denoms = denoms_opt.unwrap_or_default();
            if new_amount.is_zero() {
                denoms.retain(|s| s != denom);
            }
            Ok(denoms)
        },
    )?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount_to_reclaim.u128(), denom)],
    });

    Ok(Response::new().add_message(transfer_msg))
}
