use cosmwasm_std::{Addr, Decimal, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    red_bank::{
        Config, CreateOrUpdateConfig, Debt, InitOrUpdateAssetParams, InstantiateMsg, Market,
    },
};
use mars_utils::helpers::{
    build_send_asset_msg, option_string_to_addr, validate_native_denom, zero_address,
};

use crate::{
    error::ContractError,
    health::{
        assert_below_liq_threshold_after_withdraw, assert_below_max_ltv_after_borrow,
        get_health_and_positions,
    },
    helpers::query_asset_params,
    interest_rates::{
        apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
        get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
    },
    state::{COLLATERALS, CONFIG, DEBTS, MARKETS, OWNER, UNCOLLATERALIZED_LOAN_LIMITS},
    user::User,
};

pub const CONTRACT_NAME: &str = "crates.io:mars-red-bank";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        address_provider,
    } = msg.config;

    if address_provider.is_none() {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        address_provider: option_string_to_addr(deps.api, address_provider, zero_address())?,
    };

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
    } = new_config;

    // Update config
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

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
        reserve_factor,
        interest_rate_model,
    } = params;

    // All fields should be available
    let available = reserve_factor.is_some() && interest_rate_model.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    }

    let new_market = Market {
        denom: denom.to_string(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        interest_rate_model: interest_rate_model.unwrap(),
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
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let market_option = MARKETS.may_load(deps.storage, &denom)?;
    match market_option {
        None => Err(ContractError::AssetNotInitialized {}),
        Some(mut market) => {
            // Destructuring a struct’s fields into separate variables in order to force
            // compile error if we add more params
            let InitOrUpdateAssetParams {
                reserve_factor,
                interest_rate_model,
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
                    &env,
                    &mut market,
                    rewards_collector_addr,
                    incentives_addr,
                    response,
                )?;
            }

            let mut updated_market = Market {
                reserve_factor: reserve_factor.unwrap_or(market.reserve_factor),
                interest_rate_model: interest_rate_model.unwrap_or(market.interest_rate_model),
                ..market
            };

            updated_market.validate()?;

            if should_update_interest_rates {
                response = update_interest_rates(&env, &mut updated_market, response)?;
            }
            MARKETS.save(deps.storage, &denom, &updated_market)?;

            Ok(response.add_attribute("action", "update_asset").add_attribute("denom", denom))
        }
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
    denom: String,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut market = MARKETS.load(deps.storage, &denom)?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Params,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let params_addr = &addresses[&MarsAddressType::Params];

    let asset_params = query_asset_params(&deps.querier, params_addr, &denom)?;

    if !asset_params.red_bank.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            denom,
        });
    }

    let total_scaled_deposits = market.collateral_total_scaled;
    let total_deposits =
        get_underlying_liquidity_amount(total_scaled_deposits, &market, env.block.time.seconds())?;
    if total_deposits.checked_add(deposit_amount)? > asset_params.red_bank.deposit_cap {
        return Err(ContractError::DepositCapExceeded {
            denom,
        });
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

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }
    let deposit_amount_scaled =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;

    response = User(&info.sender).increase_collateral(
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
            MarsAddressType::Params,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let params_addr = &addresses[&MarsAddressType::Params];

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if collateral.enabled
        && withdrawer.is_borrowing(deps.storage)
        && !assert_below_liq_threshold_after_withdraw(
            &deps.as_ref(),
            &env,
            withdrawer.address(),
            oracle_addr,
            params_addr,
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

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::Oracle,
            MarsAddressType::Incentives,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Params,
        ],
    )?;
    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let params_addr = &addresses[&MarsAddressType::Params];

    let asset_params = query_asset_params(&deps.querier, params_addr, &denom)?;

    if !asset_params.red_bank.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            denom,
        });
    }

    // Load market and user state
    let mut borrow_market = MARKETS.load(deps.storage, &denom)?;

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

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if uncollateralized_loan_limit.is_zero() {
        if !assert_below_max_ltv_after_borrow(
            &deps.as_ref(),
            &env,
            borrower.address(),
            oracle_addr,
            params_addr,
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

        let addresses = address_provider::helpers::query_contract_addrs(
            deps.as_ref(),
            &config.address_provider,
            vec![MarsAddressType::Oracle, MarsAddressType::Params],
        )?;
        let oracle_addr = &addresses[&MarsAddressType::Oracle];
        let params_addr = &addresses[&MarsAddressType::Params];

        let (health, _) = get_health_and_positions(
            &deps.as_ref(),
            &env,
            user.address(),
            oracle_addr,
            params_addr,
        )?;

        if health.is_liquidatable() {
            return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
        }
    }

    Ok(Response::new()
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user)
        .add_attribute("denom", denom)
        .add_attribute("enable", enable.to_string()))
}
