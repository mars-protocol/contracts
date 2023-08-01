use std::{cmp::min, str};

use cosmwasm_std::{
    Addr, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use mars_owner::{OwnerError, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    red_bank::{
        Config, CreateOrUpdateConfig, Debt, InitOrUpdateAssetParams, InstantiateMsg, Market,
    },
};
use mars_utils::{
    helpers::{build_send_asset_msg, option_string_to_addr, validate_native_denom, zero_address},
    math,
};

use crate::{
    error::ContractError,
    health::{
        assert_below_liq_threshold_after_withdraw, assert_below_max_ltv_after_borrow,
        assert_liquidatable,
    },
    interest_rates::{
        apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
        get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
    },
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS, OWNER, UNCOLLATERALIZED_LOAN_LIMITS},
    user::User,
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        address_provider,
        close_factor,
    } = msg.config;

    // All fields should be available
    let available = address_provider.is_some() && close_factor.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        address_provider: option_string_to_addr(deps.api, address_provider, zero_address())?,
        close_factor: close_factor.unwrap(),
    };

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    Ok(Response::default())
}

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

/// Update config
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        address_provider,
        close_factor,
    } = new_config;

    // Update config
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;
    config.close_factor = close_factor.unwrap_or(config.close_factor);

    // Validate config
    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Initialize asset if not exist.
/// Initialization requires that all params are provided and there is no asset in state.
pub fn init_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    validate_native_denom(&denom)?;

    if MARKETS.may_load(deps.storage, &denom)?.is_some() {
        return Err(ContractError::AssetAlreadyInitialized {});
    }

    let new_market = create_market(env.block.time.seconds(), &denom, params)?;
    MARKETS.save(deps.storage, &denom, &new_market)?;

    Ok(Response::new().add_attribute("action", "init_asset").add_attribute("denom", denom))
}

/// Initialize new market
pub fn create_market(
    block_time: u64,
    denom: &str,
    params: InitOrUpdateAssetParams,
) -> Result<Market, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let InitOrUpdateAssetParams {
        max_loan_to_value,
        reserve_factor,
        liquidation_threshold,
        liquidation_bonus,
        interest_rate_model,
        deposit_enabled,
        borrow_enabled,
        deposit_cap,
    } = params;

    // All fields should be available
    let available = max_loan_to_value.is_some()
        && reserve_factor.is_some()
        && liquidation_threshold.is_some()
        && liquidation_bonus.is_some()
        && interest_rate_model.is_some()
        && deposit_enabled.is_some()
        && borrow_enabled.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    }

    let new_market = Market {
        denom: denom.to_string(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        max_loan_to_value: max_loan_to_value.unwrap(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        liquidation_threshold: liquidation_threshold.unwrap(),
        liquidation_bonus: liquidation_bonus.unwrap(),
        interest_rate_model: interest_rate_model.unwrap(),
        deposit_enabled: deposit_enabled.unwrap(),
        borrow_enabled: borrow_enabled.unwrap(),
        // if not specified, deposit cap is set to unlimited
        deposit_cap: deposit_cap.unwrap_or(Uint128::MAX),
    };

    new_market.validate()?;

    Ok(new_market)
}

/// Update asset with new params.
pub fn update_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    if OWNER.is_owner(deps.storage, &info.sender)? {
        update_asset_by_owner(deps, &env, &denom, params)
    } else if OWNER.is_emergency_owner(deps.storage, &info.sender)? {
        update_asset_by_emergency_owner(deps, &denom, params)
    } else {
        Err(OwnerError::NotOwner {}.into())
    }
}

fn update_asset_by_owner(
    deps: DepsMut,
    env: &Env,
    denom: &str,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    let market_option = MARKETS.may_load(deps.storage, denom)?;
    match market_option {
        None => Err(ContractError::AssetNotInitialized {}),
        Some(mut market) => {
            // Destructuring a struct’s fields into separate variables in order to force
            // compile error if we add more params
            let InitOrUpdateAssetParams {
                max_loan_to_value,
                reserve_factor,
                liquidation_threshold,
                liquidation_bonus,
                interest_rate_model,
                deposit_enabled,
                borrow_enabled,
                deposit_cap,
            } = params;

            // If reserve factor or interest rates are updated we update indexes with
            // current values before applying the change to prevent applying this
            // new params to a period where they were not valid yet. Interests rates are
            // recalculated after changes are applied.
            let should_update_interest_rates = (reserve_factor.is_some()
                && reserve_factor.unwrap() != market.reserve_factor)
                || interest_rate_model.is_some();

            let mut response = Response::new();

            if should_update_interest_rates {
                let config = CONFIG.load(deps.storage)?;
                let addresses = address_provider::helpers::query_contract_addrs(
                    deps.as_ref(),
                    &config.address_provider,
                    vec![MarsAddressType::Incentives, MarsAddressType::RewardsCollector],
                )?;
                let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
                let incentives_addr = &addresses[&MarsAddressType::Incentives];

                response = apply_accumulated_interests(
                    deps.storage,
                    env,
                    &mut market,
                    rewards_collector_addr,
                    incentives_addr,
                    response,
                )?;
            }

            let mut updated_market = Market {
                max_loan_to_value: max_loan_to_value.unwrap_or(market.max_loan_to_value),
                reserve_factor: reserve_factor.unwrap_or(market.reserve_factor),
                liquidation_threshold: liquidation_threshold
                    .unwrap_or(market.liquidation_threshold),
                liquidation_bonus: liquidation_bonus.unwrap_or(market.liquidation_bonus),
                interest_rate_model: interest_rate_model.unwrap_or(market.interest_rate_model),
                deposit_enabled: deposit_enabled.unwrap_or(market.deposit_enabled),
                borrow_enabled: borrow_enabled.unwrap_or(market.borrow_enabled),
                deposit_cap: deposit_cap.unwrap_or(market.deposit_cap),
                ..market
            };

            updated_market.validate()?;

            if should_update_interest_rates {
                response = update_interest_rates(env, &mut updated_market, response)?;
            }
            MARKETS.save(deps.storage, denom, &updated_market)?;

            Ok(response.add_attribute("action", "update_asset").add_attribute("denom", denom))
        }
    }
}

/// Emergency owner can only DISABLE BORROWING.
fn update_asset_by_emergency_owner(
    deps: DepsMut,
    denom: &str,
    params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    if let Some(mut market) = MARKETS.may_load(deps.storage, denom)? {
        match params.borrow_enabled {
            Some(borrow_enabled) if !borrow_enabled => {
                market.borrow_enabled = borrow_enabled;
                MARKETS.save(deps.storage, denom, &market)?;

                Ok(Response::new()
                    .add_attribute("action", "emergency_update_asset")
                    .add_attribute("denom", denom))
            }
            _ => Err(MarsError::Unauthorized {}.into()),
        }
    } else {
        Err(ContractError::AssetNotInitialized {})
    }
}

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

/// Execute deposits
pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    on_behalf_of: Option<String>,
    denom: String,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_addr: Addr;
    let user = if let Some(address) = on_behalf_of {
        user_addr = deps.api.addr_validate(&address)?;
        User(&user_addr)
    } else {
        User(&info.sender)
    };

    let mut market = MARKETS.load(deps.storage, &denom)?;
    if !market.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            denom,
        });
    }

    let total_scaled_deposits = market.collateral_total_scaled;
    let total_deposits =
        get_underlying_liquidity_amount(total_scaled_deposits, &market, env.block.time.seconds())?;
    if total_deposits.checked_add(deposit_amount)? > market.deposit_cap {
        return Err(ContractError::DepositCapExceeded {
            denom,
        });
    }

    let mut response = Response::new();

    let config = CONFIG.load(deps.storage)?;

    // update indexes and interest rates
    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::Incentives, MarsAddressType::RewardsCollector],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }
    let deposit_amount_scaled =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;

    response = user.increase_collateral(
        deps.storage,
        &market,
        deposit_amount_scaled,
        incentives_addr,
        response,
    )?;

    market.increase_collateral(deposit_amount_scaled)?;

    response = update_interest_rates(&env, &mut market, response)?;

    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(response
        .add_attribute("action", "deposit")
        .add_attribute("sender", &info.sender)
        .add_attribute("on_behalf_of", user)
        .add_attribute("denom", denom)
        .add_attribute("amount", deposit_amount)
        .add_attribute("amount_scaled", deposit_amount_scaled))
}

/// Burns sent maAsset in exchange of underlying asset
pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    amount: Option<Uint128>,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let withdrawer = User(&info.sender);

    let mut market = MARKETS.load(deps.storage, &denom)?;

    let collateral = withdrawer.collateral(deps.storage, &denom)?;
    let withdrawer_balance_scaled_before = collateral.amount_scaled;

    if withdrawer_balance_scaled_before.is_zero() {
        return Err(ContractError::UserNoCollateralBalance {
            user: withdrawer.into(),
            denom,
        });
    }

    let withdrawer_balance_before = get_underlying_liquidity_amount(
        withdrawer_balance_scaled_before,
        &market,
        env.block.time.seconds(),
    )?;

    let withdraw_amount = match amount {
        // Check user has sufficient balance to send back
        Some(amount) if amount.is_zero() || amount > withdrawer_balance_before => {
            return Err(ContractError::InvalidWithdrawAmount {
                denom,
            });
        }
        Some(amount) => amount,
        // If no amount is specified, the full balance is withdrawn
        None => withdrawer_balance_before,
    };

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if collateral.enabled
        && withdrawer.is_borrowing(deps.storage)
        && !assert_below_liq_threshold_after_withdraw(
            &deps.as_ref(),
            &env,
            withdrawer.address(),
            oracle_addr,
            &denom,
            withdraw_amount,
        )?
    {
        return Err(ContractError::InvalidHealthFactorAfterWithdraw {});
    }

    let mut response = Response::new();

    // update indexes and interest rates
    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    // reduce the withdrawer's scaled collateral amount
    let withdrawer_balance_after = withdrawer_balance_before.checked_sub(withdraw_amount)?;
    let withdrawer_balance_scaled_after =
        get_scaled_liquidity_amount(withdrawer_balance_after, &market, env.block.time.seconds())?;

    let withdraw_amount_scaled =
        withdrawer_balance_scaled_before.checked_sub(withdrawer_balance_scaled_after)?;

    response = withdrawer.decrease_collateral(
        deps.storage,
        &market,
        withdraw_amount_scaled,
        incentives_addr,
        response,
    )?;

    market.decrease_collateral(withdraw_amount_scaled)?;

    response = update_interest_rates(&env, &mut market, response)?;

    MARKETS.save(deps.storage, &denom, &market)?;

    // send underlying asset to user or another recipient
    let recipient_addr = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
    } else {
        withdrawer.address().clone()
    };

    Ok(response
        .add_message(build_send_asset_msg(&recipient_addr, &denom, withdraw_amount))
        .add_attribute("action", "withdraw")
        .add_attribute("sender", withdrawer)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("denom", denom)
        .add_attribute("amount", withdraw_amount)
        .add_attribute("amount_scaled", withdraw_amount_scaled))
}

/// Add debt for the borrower and send the borrowed funds
pub fn borrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    borrow_amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let borrower = User(&info.sender);

    // Load market and user state
    let mut borrow_market = MARKETS.load(deps.storage, &denom)?;

    if !borrow_market.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            denom,
        });
    }

    let collateral_balance_before = get_underlying_liquidity_amount(
        borrow_market.collateral_total_scaled,
        &borrow_market,
        env.block.time.seconds(),
    )?;

    // Cannot borrow zero amount or more than available collateral
    if borrow_amount.is_zero() || borrow_amount > collateral_balance_before {
        return Err(ContractError::InvalidBorrowAmount {
            denom,
        });
    }

    let uncollateralized_loan_limit = borrower.uncollateralized_loan_limit(deps.storage, &denom)?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if uncollateralized_loan_limit.is_zero() {
        if !assert_below_max_ltv_after_borrow(
            &deps.as_ref(),
            &env,
            borrower.address(),
            oracle_addr,
            &denom,
            borrow_amount,
        )? {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    } else {
        // Uncollateralized loan: check borrow amount plus debt does not exceed uncollateralized loan limit
        uncollateralized_debt = true;

        let debt_amount_scaled = borrower.debt_amount_scaled(deps.storage, &denom)?;

        let asset_market = MARKETS.load(deps.storage, &denom)?;
        let debt_amount = get_underlying_debt_amount(
            debt_amount_scaled,
            &asset_market,
            env.block.time.seconds(),
        )?;

        let debt_after_borrow = debt_amount.checked_add(borrow_amount)?;
        if debt_after_borrow > uncollateralized_loan_limit {
            return Err(ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});
        }
    }

    let mut response = Response::new();

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut borrow_market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    // Set new debt
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;

    borrow_market.increase_debt(borrow_amount_scaled)?;
    borrower.increase_debt(deps.storage, &denom, borrow_amount_scaled, uncollateralized_debt)?;

    response = update_interest_rates(&env, &mut borrow_market, response)?;
    MARKETS.save(deps.storage, &denom, &borrow_market)?;

    // Send borrow amount to borrower or another recipient
    let recipient_addr = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?
    } else {
        borrower.address().clone()
    };

    Ok(response
        .add_message(build_send_asset_msg(&recipient_addr, &denom, borrow_amount))
        .add_attribute("action", "borrow")
        .add_attribute("sender", borrower)
        .add_attribute("recipient", recipient_addr)
        .add_attribute("denom", denom)
        .add_attribute("amount", borrow_amount)
        .add_attribute("amount_scaled", borrow_amount_scaled))
}

/// Handle the repay of native tokens. Refund extra funds if they exist
pub fn repay(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    on_behalf_of: Option<String>,
    denom: String,
    repay_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_addr: Addr;
    let user = if let Some(address) = on_behalf_of {
        user_addr = deps.api.addr_validate(&address)?;
        let user = User(&user_addr);
        // Uncollateralized loans should not have 'on behalf of' because it creates accounting complexity for them
        if !user.uncollateralized_loan_limit(deps.storage, &denom)?.is_zero() {
            return Err(ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {});
        }
        user
    } else {
        User(&info.sender)
    };

    // Check new debt
    let debt = DEBTS
        .may_load(deps.storage, (user.address(), &denom))?
        .ok_or(ContractError::CannotRepayZeroDebt {})?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::Incentives, MarsAddressType::RewardsCollector],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];

    let mut market = MARKETS.load(deps.storage, &denom)?;

    let mut response = Response::new();

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    let debt_amount_scaled_before = debt.amount_scaled;
    let debt_amount_before =
        get_underlying_debt_amount(debt.amount_scaled, &market, env.block.time.seconds())?;

    // If repay amount exceeds debt, refund any excess amounts
    let mut refund_amount = Uint128::zero();
    let mut debt_amount_after = Uint128::zero();
    if repay_amount > debt_amount_before {
        refund_amount = repay_amount - debt_amount_before;
        let refund_msg = build_send_asset_msg(&info.sender, &denom, refund_amount);
        response = response.add_message(refund_msg);
    } else {
        debt_amount_after = debt_amount_before - repay_amount;
    }

    let debt_amount_scaled_after =
        get_scaled_debt_amount(debt_amount_after, &market, env.block.time.seconds())?;

    let debt_amount_scaled_delta =
        debt_amount_scaled_before.checked_sub(debt_amount_scaled_after)?;

    market.decrease_debt(debt_amount_scaled_delta)?;
    user.decrease_debt(deps.storage, &denom, debt_amount_scaled_delta)?;

    response = update_interest_rates(&env, &mut market, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(response
        .add_attribute("action", "repay")
        .add_attribute("sender", &info.sender)
        .add_attribute("on_behalf_of", user)
        .add_attribute("denom", denom)
        .add_attribute("amount", repay_amount.checked_sub(refund_amount)?)
        .add_attribute("amount_scaled", debt_amount_scaled_delta))
}

/// Execute loan liquidations on under-collateralized loans
pub fn liquidate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_denom: String,
    debt_denom: String,
    user_addr: Addr,
    sent_debt_amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let user = User(&user_addr);
    // The recipient address for receiving underlying collateral
    let recipient_addr = option_string_to_addr(deps.api, recipient, info.sender.clone())?;
    let recipient = User(&recipient_addr);

    // 1. Validate liquidation
    // If user (contract) has a positive uncollateralized limit then the user
    // cannot be liquidated
    if !user.uncollateralized_loan_limit(deps.storage, &debt_denom)?.is_zero() {
        return Err(ContractError::CannotLiquidateWhenPositiveUncollateralizedLoanLimit {});
    };

    // check if the user has enabled the collateral asset as collateral
    let user_collateral = COLLATERALS
        .may_load(deps.storage, (&user_addr, &collateral_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoCollateralBalance {})?;
    if !user_collateral.enabled {
        return Err(ContractError::CannotLiquidateWhenCollateralUnset {
            denom: collateral_denom,
        });
    }

    // check if user has available collateral in specified collateral asset to be liquidated
    let collateral_market = MARKETS.load(deps.storage, &collateral_denom)?;

    // check if user has outstanding debt in the deposited asset that needs to be repayed
    let user_debt = DEBTS
        .may_load(deps.storage, (&user_addr, &debt_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoDebtBalance {})?;

    // 2. Compute health factor
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];

    let (liquidatable, assets_positions) =
        assert_liquidatable(&deps.as_ref(), &env, &user_addr, oracle_addr)?;

    if !liquidatable {
        return Err(ContractError::CannotLiquidateHealthyPosition {});
    }

    let collateral_and_debt_are_the_same_asset = debt_denom == collateral_denom;

    let debt_market = if !collateral_and_debt_are_the_same_asset {
        MARKETS.load(deps.storage, &debt_denom)?
    } else {
        collateral_market.clone()
    };

    // 3. Compute debt to repay and collateral to liquidate
    let collateral_price = assets_positions
        .get(&collateral_denom)
        .ok_or(ContractError::CannotLiquidateWhenNoCollateralBalance {})?
        .asset_price;
    let debt_price = assets_positions
        .get(&debt_denom)
        .ok_or(ContractError::CannotLiquidateWhenNoDebtBalance {})?
        .asset_price;

    let mut response = Response::new();

    let user_debt_amount =
        get_underlying_debt_amount(user_debt.amount_scaled, &debt_market, block_time)?;

    let (
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_to_liquidate_scaled,
        refund_amount,
    ) = liquidation_compute_amounts(
        user_collateral.amount_scaled,
        user_debt_amount,
        sent_debt_amount,
        &collateral_market,
        collateral_price,
        debt_price,
        block_time,
        config.close_factor,
    )?;

    // 4. Transfer collateral shares from the user to the liquidator
    response = user.decrease_collateral(
        deps.storage,
        &collateral_market,
        collateral_amount_to_liquidate_scaled,
        incentives_addr,
        response,
    )?;
    response = recipient.increase_collateral(
        deps.storage,
        &collateral_market,
        collateral_amount_to_liquidate_scaled,
        incentives_addr,
        response,
    )?;

    // 5. Reduce the user's debt shares
    let user_debt_amount_after = user_debt_amount.checked_sub(debt_amount_to_repay)?;
    let user_debt_amount_scaled_after =
        get_scaled_debt_amount(user_debt_amount_after, &debt_market, block_time)?;

    // Compute delta so it can be substracted to total debt
    let debt_amount_scaled_delta =
        user_debt.amount_scaled.checked_sub(user_debt_amount_scaled_after)?;

    user.decrease_debt(deps.storage, &debt_denom, debt_amount_scaled_delta)?;

    let debt_market_debt_total_scaled_after =
        debt_market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

    // 6. Update markets depending on whether the collateral and debt markets are the same
    // and whether the liquidator receives coins (no change in liquidity) or underlying asset
    // (changes liquidity)
    if collateral_and_debt_are_the_same_asset {
        // NOTE: for the sake of clarity copy attributes from collateral market and
        // give generic naming. Debt market could have been used as well
        let mut asset_market_after = collateral_market;
        let denom = &collateral_denom;

        response = apply_accumulated_interests(
            deps.storage,
            &env,
            &mut asset_market_after,
            rewards_collector_addr,
            incentives_addr,
            response,
        )?;

        asset_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        response = update_interest_rates(&env, &mut asset_market_after, response)?;

        MARKETS.save(deps.storage, denom, &asset_market_after)?;
    } else {
        let mut debt_market_after = debt_market;

        response = apply_accumulated_interests(
            deps.storage,
            &env,
            &mut debt_market_after,
            rewards_collector_addr,
            incentives_addr,
            response,
        )?;

        debt_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        response = update_interest_rates(&env, &mut debt_market_after, response)?;

        MARKETS.save(deps.storage, &debt_denom, &debt_market_after)?;
    }

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    if !refund_amount.is_zero() {
        response =
            response.add_message(build_send_asset_msg(&info.sender, &debt_denom, refund_amount));
    }

    Ok(response
        .add_attribute("action", "liquidate")
        .add_attribute("user", user)
        .add_attribute("liquidator", info.sender.to_string())
        .add_attribute("recipient", recipient)
        .add_attribute("collateral_denom", collateral_denom)
        .add_attribute("collateral_amount", collateral_amount_to_liquidate)
        .add_attribute("collateral_amount_scaled", collateral_amount_to_liquidate_scaled)
        .add_attribute("debt_denom", debt_denom)
        .add_attribute("debt_amount", debt_amount_to_repay)
        .add_attribute("debt_amount_scaled", debt_amount_scaled_delta))
}

/// Computes debt to repay (in debt asset),
/// collateral to liquidate (in collateral asset) and
/// amount to refund the liquidator (in debt asset)
pub fn liquidation_compute_amounts(
    user_collateral_amount_scaled: Uint128,
    user_debt_amount: Uint128,
    sent_debt_amount: Uint128,
    collateral_market: &Market,
    collateral_price: Decimal,
    debt_price: Decimal,
    block_time: u64,
    close_factor: Decimal,
) -> StdResult<(Uint128, Uint128, Uint128, Uint128)> {
    // Debt: Only up to a fraction of the total debt (determined by the close factor) can be
    // repayed.
    let mut debt_amount_to_repay = min(sent_debt_amount, close_factor * user_debt_amount);

    // Collateral: debt to repay in base asset times the liquidation bonus
    let mut collateral_amount_to_liquidate = math::divide_uint128_by_decimal(
        debt_amount_to_repay * debt_price * (Decimal::one() + collateral_market.liquidation_bonus),
        collateral_price,
    )?;
    let mut collateral_amount_to_liquidate_scaled =
        get_scaled_liquidity_amount(collateral_amount_to_liquidate, collateral_market, block_time)?;

    // If collateral amount to liquidate is higher than user_collateral_balance,
    // liquidate the full balance and adjust the debt amount to repay accordingly
    if collateral_amount_to_liquidate_scaled > user_collateral_amount_scaled {
        collateral_amount_to_liquidate_scaled = user_collateral_amount_scaled;
        collateral_amount_to_liquidate = get_underlying_liquidity_amount(
            collateral_amount_to_liquidate_scaled,
            collateral_market,
            block_time,
        )?;
        debt_amount_to_repay = math::divide_uint128_by_decimal(
            math::divide_uint128_by_decimal(
                collateral_amount_to_liquidate * collateral_price,
                debt_price,
            )?,
            Decimal::one() + collateral_market.liquidation_bonus,
        )?;
    }

    // In some edges scenarios:
    // - if debt_amount_to_repay = 0, some liquidators could drain collaterals and all their coins
    // would be refunded, i.e.: without spending coins.
    // - if collateral_amount_to_liquidate is 0, some users could liquidate without receiving collaterals
    // in return.
    if (!collateral_amount_to_liquidate.is_zero() && debt_amount_to_repay.is_zero())
        || (collateral_amount_to_liquidate.is_zero() && !debt_amount_to_repay.is_zero())
    {
        return Err(StdError::generic_err(
            format!("Can't process liquidation. Invalid collateral_amount_to_liquidate ({collateral_amount_to_liquidate}) and debt_amount_to_repay ({debt_amount_to_repay})")
        ));
    }

    let refund_amount = sent_debt_amount - debt_amount_to_repay;

    Ok((
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_to_liquidate_scaled,
        refund_amount,
    ))
}

/// Update (enable / disable) collateral asset for specific user
pub fn update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    enable: bool,
) -> Result<Response, ContractError> {
    let user = User(&info.sender);

    let mut collateral =
        COLLATERALS.may_load(deps.storage, (user.address(), &denom))?.ok_or_else(|| {
            ContractError::UserNoCollateralBalance {
                user: user.into(),
                denom: denom.clone(),
            }
        })?;

    let previously_enabled = collateral.enabled;

    collateral.enabled = enable;
    COLLATERALS.save(deps.storage, (user.address(), &denom), &collateral)?;

    // if the collateral was previously enabled, but is not disabled, it is necessary to ensure the
    // user is not liquidatable after disabling
    if previously_enabled && !enable {
        let config = CONFIG.load(deps.storage)?;
        let oracle_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &config.address_provider,
            MarsAddressType::Oracle,
        )?;

        let (liquidatable, _) =
            assert_liquidatable(&deps.as_ref(), &env, user.address(), &oracle_addr)?;

        if liquidatable {
            return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
        }
    }

    Ok(Response::new()
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user)
        .add_attribute("denom", denom)
        .add_attribute("enable", enable.to_string()))
}
