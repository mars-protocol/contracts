use std::str;

use cosmwasm_std::{Addr, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128};

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::{build_send_asset_msg, option_string_to_addr, zero_address};
use mars_outpost::math;
use mars_outpost::red_bank::{
    init_interest_rate_model, Config, CreateOrUpdateConfig, Debt, InitOrUpdateAssetParams,
    InstantiateMsg, Market, UserHealthStatus,
};

use crate::accounts::get_user_position;
use crate::error::ContractError;
use crate::events::build_collateral_position_changed_event;
use crate::interest_rates::{
    apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
    get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
};
use crate::state::{
    deduct_collateral, deduct_debt, increment_collateral, increment_debt, COLLATERALS, CONFIG,
    DEBTS, MARKETS, UNCOLLATERALIZED_LOAN_LIMITS,
};

pub fn instantiate(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        address_provider_address,
        close_factor,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some() && address_provider_address.is_some() && close_factor.is_some();
    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        owner: option_string_to_addr(deps.api, owner, zero_address())?,
        address_provider_address: option_string_to_addr(
            deps.api,
            address_provider_address,
            zero_address(),
        )?,
        close_factor: close_factor.unwrap(),
    };

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        address_provider_address,
        close_factor,
    } = new_config;

    // Update config
    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address =
        option_string_to_addr(deps.api, address_provider_address, config.address_provider_address)?;
    config.close_factor = close_factor.unwrap_or(config.close_factor);

    // Validate config
    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "update_config");
    Ok(res)
}

/// Initialize asset if not exist.
/// Initialization requires that all params are provided and there is no asset in state.
pub fn init_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    asset_params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let market_option = MARKETS.may_load(deps.storage, &denom)?;
    match market_option {
        None => {
            let new_market = create_market(env.block.time.seconds(), &denom, asset_params)?;

            // Save new market
            MARKETS.save(deps.storage, &denom, &new_market)?;

            Ok(Response::new().add_attribute("action", "init_asset").add_attribute("denom", denom))
        }
        Some(_) => Err(ContractError::AssetAlreadyInitialized {}),
    }
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
        initial_borrow_rate: borrow_rate,
        max_loan_to_value,
        reserve_factor,
        liquidation_threshold,
        liquidation_bonus,
        interest_rate_model_params,
        active,
        deposit_enabled,
        borrow_enabled,
    } = params;

    // All fields should be available
    let available = borrow_rate.is_some()
        && max_loan_to_value.is_some()
        && reserve_factor.is_some()
        && liquidation_threshold.is_some()
        && liquidation_bonus.is_some()
        && interest_rate_model_params.is_some()
        && active.is_some()
        && deposit_enabled.is_some()
        && borrow_enabled.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    }

    let new_market = Market {
        denom: denom.to_string(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: borrow_rate.unwrap(),
        liquidity_rate: Decimal::zero(),
        max_loan_to_value: max_loan_to_value.unwrap(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        liquidation_threshold: liquidation_threshold.unwrap(),
        liquidation_bonus: liquidation_bonus.unwrap(),
        interest_rate_model: init_interest_rate_model(
            interest_rate_model_params.unwrap(),
            block_time,
        )?,
        active: active.unwrap(),
        deposit_enabled: deposit_enabled.unwrap(),
        borrow_enabled: borrow_enabled.unwrap(),
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
    asset_params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let market_option = MARKETS.may_load(deps.storage, &denom)?;
    match market_option {
        None => Err(ContractError::AssetNotInitialized {}),
        Some(mut market) => {
            // Destructuring a struct’s fields into separate variables in order to force
            // compile error if we add more params
            let InitOrUpdateAssetParams {
                initial_borrow_rate: _,
                max_loan_to_value,
                reserve_factor,
                liquidation_threshold,
                liquidation_bonus,
                interest_rate_model_params,
                active,
                deposit_enabled,
                borrow_enabled,
            } = asset_params;

            // If reserve factor or interest rates are updated we update indexes with
            // current values before applying the change to prevent applying this
            // new params to a period where they were not valid yet. Interests rates are
            // recalculated after changes are applied.
            let should_update_interest_rates = (reserve_factor.is_some()
                && reserve_factor.unwrap() != market.reserve_factor)
                || interest_rate_model_params.is_some();

            if should_update_interest_rates {
                let protocol_rewards_collector_address = address_provider::helpers::query_address(
                    deps.as_ref(),
                    &config.address_provider_address,
                    MarsContract::ProtocolRewardsCollector,
                )?;
                apply_accumulated_interests(
                    deps.storage,
                    &env,
                    &protocol_rewards_collector_address,
                    &mut market,
                )?;
            }

            let mut updated_market = Market {
                max_loan_to_value: max_loan_to_value.unwrap_or(market.max_loan_to_value),
                reserve_factor: reserve_factor.unwrap_or(market.reserve_factor),
                liquidation_threshold: liquidation_threshold
                    .unwrap_or(market.liquidation_threshold),
                liquidation_bonus: liquidation_bonus.unwrap_or(market.liquidation_bonus),
                active: active.unwrap_or(market.active),
                deposit_enabled: deposit_enabled.unwrap_or(market.deposit_enabled),
                borrow_enabled: borrow_enabled.unwrap_or(market.borrow_enabled),
                ..market
            };

            if let Some(params) = interest_rate_model_params {
                updated_market.interest_rate_model =
                    init_interest_rate_model(params, env.block.time.seconds())?;
            }

            updated_market.validate()?;

            let mut events = vec![];
            if should_update_interest_rates {
                update_interest_rates(
                    &deps,
                    &env,
                    &mut updated_market,
                    Uint128::zero(),
                    &denom,
                    &mut events,
                )?;
            }
            MARKETS.save(deps.storage, &denom, &updated_market)?;

            Ok(Response::new()
                .add_events(events)
                .add_attribute("action", "update_asset")
                .add_attribute("denom", &denom))
        }
    }
}

pub fn update_uncollateralized_loan_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_address: Addr,
    denom: String,
    new_limit: Uint128,
) -> Result<Response, ContractError> {
    // Get config
    let config = CONFIG.load(deps.storage)?;

    // Only owner can do this
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    // Check that the user has no collateralized debt
    //
    // the error condition is that for the given denom, the user currently:
    //
    // - DOES NOT have an uncollateral loan limit
    // - DOES have a non-zero amount of collateral deposited in the money market
    let current_uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (&user_address, &denom))?
        .unwrap_or_else(Uint128::zero);
    let current_collateral_amount_scaled = COLLATERALS
        .may_load(deps.storage, (&user_address, &denom))?
        .map(|collateral| collateral.amount_scaled)
        .unwrap_or_else(Uint128::zero);
    if current_uncollateralized_loan_limit.is_zero() && !current_collateral_amount_scaled.is_zero()
    {
        return Err(ContractError::UserHasCollateralizedDebt {});
    }

    UNCOLLATERALIZED_LOAN_LIMITS.save(deps.storage, (&user_address, &denom), &new_limit)?;

    DEBTS.update(
        deps.storage,
        (&user_address, &denom),
        |debt_opt: Option<Debt>| -> StdResult<_> {
            let mut debt = debt_opt.unwrap_or(Debt {
                amount_scaled: Uint128::zero(),
                uncollateralized: false,
            });
            // if limit == 0 then uncollateralized = false, otherwise uncollateralized = true
            debt.uncollateralized = !new_limit.is_zero();
            Ok(debt)
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "update_uncollateralized_loan_limit")
        .add_attribute("user", user_address.as_str())
        .add_attribute("denom", denom)
        .add_attribute("new_allowance", new_limit.to_string()))
}

pub fn deposit(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender_address: Addr,
    on_behalf_of: Option<String>,
    denom: String,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_address = if let Some(address) = on_behalf_of {
        deps.api.addr_validate(&address)?
    } else {
        sender_address.clone()
    };

    let mut market = MARKETS.load(deps.storage, &denom)?;
    if !market.active {
        return Err(ContractError::MarketNotActive {
            denom,
        });
    }
    if !market.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            denom,
        });
    }

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(ContractError::InvalidDepositAmount {
            denom,
        });
    }

    // Update the depositor's collateral position
    let mut events = vec![];
    let deposit_amount_scaled =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;
    increment_collateral(
        deps.storage,
        &user_address,
        &denom,
        deposit_amount_scaled,
        true,
        Some(&mut events),
    )?;

    // Update market: total collateral amount, indexes, and interest rates
    let config = CONFIG.load(deps.storage)?;
    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;
    apply_accumulated_interests(
        deps.storage,
        &env,
        &protocol_rewards_collector_address,
        &mut market,
    )?;
    update_interest_rates(&deps, &env, &mut market, Uint128::zero(), &denom, &mut events)?;
    market.collateral_total_scaled += deposit_amount_scaled;
    MARKETS.save(deps.storage, &denom, &market)?;

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }

    Ok(Response::new()
        .add_events(events)
        .add_attribute("action", "deposit")
        .add_attribute("denom", denom)
        .add_attribute("sender", sender_address)
        .add_attribute("user", user_address.as_str())
        .add_attribute("amount", deposit_amount))
}

/// Burns sent maAsset in exchange of underlying asset
pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    amount: Option<Uint128>,
    recipient_address: Option<String>,
) -> Result<Response, ContractError> {
    let withdrawer_addr = info.sender;

    let mut market = MARKETS.load(deps.storage, &denom)?;

    if !market.active {
        return Err(ContractError::MarketNotActive {
            denom,
        });
    }

    let withdrawer_balance_scaled_before = COLLATERALS
        .may_load(deps.storage, (&withdrawer_addr, &denom))?
        .map(|collateral| collateral.amount_scaled)
        .unwrap_or_else(Uint128::zero);
    let withdrawer_balance_before = get_underlying_liquidity_amount(
        withdrawer_balance_scaled_before,
        &market,
        env.block.time.seconds(),
    )?;

    if withdrawer_balance_scaled_before.is_zero() {
        return Err(ContractError::UserNoBalance {
            denom,
        });
    }

    let withdraw_amount = match amount {
        Some(amount) => {
            // Check user has sufficient balance to send back
            if amount.is_zero() || amount > withdrawer_balance_before {
                return Err(ContractError::InvalidWithdrawAmount {
                    denom,
                });
            };
            amount
        }
        None => {
            // If no amount is specified, the full balance is withdrawn
            withdrawer_balance_before
        }
    };

    let config = CONFIG.load(deps.storage)?;
    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_address = &addresses[&MarsContract::Oracle];

    let asset_as_collateral = !withdrawer_balance_scaled_before.is_zero();

    // the user is borrowing if, in the DEBTS map, there is at least one denom stored under the
    // withdrawer address prefix
    let user_is_borrowing = DEBTS
        .prefix(&withdrawer_addr)
        .range(deps.storage, None, None, Order::Ascending)
        .next()
        .is_some();

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if asset_as_collateral && user_is_borrowing {
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &withdrawer_addr,
            oracle_address,
        )?;

        let withdraw_asset_price = user_position.get_asset_price(&denom)?;

        let withdraw_amount_in_base_asset = withdraw_amount * withdraw_asset_price;

        let weighted_liquidation_threshold_in_base_asset_after_withdraw = user_position
            .weighted_liquidation_threshold_in_base_asset
            .checked_sub(withdraw_amount_in_base_asset * market.liquidation_threshold)?;
        let health_factor_after_withdraw = Decimal::from_ratio(
            weighted_liquidation_threshold_in_base_asset_after_withdraw,
            user_position.total_collateralized_debt_in_base_asset,
        );
        if health_factor_after_withdraw < Decimal::one() {
            return Err(ContractError::InvalidHealthFactorAfterWithdraw {});
        }
    }

    let mut events = vec![];

    // reduce the withdrawer's scaled collateral amount
    let withdrawer_balance_after = withdrawer_balance_before.checked_sub(withdraw_amount)?;
    let withdrawer_balance_scaled_after =
        get_scaled_liquidity_amount(withdrawer_balance_after, &market, env.block.time.seconds())?;
    let withdraw_amount_scaled =
        withdrawer_balance_scaled_before.checked_sub(withdrawer_balance_scaled_after)?;
    deduct_collateral(
        deps.storage,
        &withdrawer_addr,
        &denom,
        withdraw_amount_scaled,
        Some(&mut events),
    )?;

    // update market: total collateral amount, indexes, and interest rates
    apply_accumulated_interests(
        deps.storage,
        &env,
        protocol_rewards_collector_address,
        &mut market,
    )?;
    update_interest_rates(&deps, &env, &mut market, withdraw_amount, &denom, &mut events)?;
    market.collateral_total_scaled -= withdraw_amount_scaled;
    MARKETS.save(deps.storage, &denom, &market)?;

    // send underlying asset to user or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        withdrawer_addr.clone()
    };

    Ok(Response::new()
        .add_message(build_send_asset_msg(&recipient_address, &denom, withdraw_amount))
        .add_events(events)
        .add_attribute("action", "withdraw")
        .add_attribute("denom", denom)
        .add_attribute("user", withdrawer_addr.as_str())
        .add_attribute("recipient", recipient_address.as_str())
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
    recipient_address: Option<String>,
) -> Result<Response, ContractError> {
    let borrower_address = info.sender;

    // Cannot borrow zero amount
    if borrow_amount.is_zero() {
        return Err(ContractError::InvalidBorrowAmount {
            denom,
        });
    }

    // Load market and user state
    let mut borrow_market = MARKETS.load(deps.storage, &denom)?;

    if !borrow_market.active {
        return Err(ContractError::MarketNotActive {
            denom,
        });
    }
    if !borrow_market.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            denom,
        });
    }

    let uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (&borrower_address, &denom))?
        .unwrap_or_else(Uint128::zero);

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_address = &addresses[&MarsContract::Oracle];

    // Check if user can borrow specified amount
    let uncollateralized = !uncollateralized_loan_limit.is_zero();
    if uncollateralized {
        // Collateralized loan: check max ltv is not exceeded
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &borrower_address,
            oracle_address,
        )?;

        let borrow_asset_price = match user_position.get_asset_price(&denom) {
            Ok(price) => price,
            Err(_) => {
                mars_outpost::oracle::helpers::query_price(&deps.querier, oracle_address, &denom)?
            }
        };

        let borrow_amount_in_base_asset = borrow_amount * borrow_asset_price;

        let total_debt_in_base_asset_after_borrow =
            user_position.total_debt_in_base_asset.checked_add(borrow_amount_in_base_asset)?;
        if total_debt_in_base_asset_after_borrow > user_position.max_debt_in_base_asset {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    }
    // Uncollateralized loan: check borrow amount plus debt does not exceed uncollateralized loan limit
    {
        let borrower_debt =
            DEBTS.may_load(deps.storage, (&borrower_address, &denom))?.unwrap_or(Debt {
                amount_scaled: Uint128::zero(),
                uncollateralized,
            });

        let asset_market = MARKETS.load(deps.storage, &denom)?;
        let debt_amount = get_underlying_debt_amount(
            borrower_debt.amount_scaled,
            &asset_market,
            env.block.time.seconds(),
        )?;

        let debt_after_borrow = debt_amount.checked_add(borrow_amount)?;
        if debt_after_borrow > uncollateralized_loan_limit {
            return Err(ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {});
        }
    }

    let mut events = vec![];

    apply_accumulated_interests(
        deps.storage,
        &env,
        protocol_rewards_collector_address,
        &mut borrow_market,
    )?;

    // Update user debt position
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;
    increment_debt(
        deps.storage,
        &borrower_address,
        &denom,
        borrow_amount_scaled,
        uncollateralized,
        Some(&mut events),
    )?;

    // Update market
    borrow_market.debt_total_scaled += borrow_amount_scaled;
    update_interest_rates(&deps, &env, &mut borrow_market, borrow_amount, &denom, &mut events)?;
    MARKETS.save(deps.storage, &denom, &borrow_market)?;

    // Send borrow amount to borrower or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        borrower_address.clone()
    };

    Ok(Response::new()
        .add_message(build_send_asset_msg(&recipient_address, &denom, borrow_amount))
        .add_events(events)
        .add_attribute("action", "borrow")
        .add_attribute("denom", denom)
        .add_attribute("user", borrower_address.as_str())
        .add_attribute("recipient", recipient_address.as_str())
        .add_attribute("amount", borrow_amount))
}

/// Handle the repay of native tokens. Refund extra funds if they exist
pub fn repay(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender_address: Addr,
    on_behalf_of: Option<String>,
    denom: String,
    repay_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_address = if let Some(address) = on_behalf_of {
        let on_behalf_of_addr = deps.api.addr_validate(&address)?;
        // Uncollateralized loans should not have 'on behalf of' because it creates accounting complexity for them
        match UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&on_behalf_of_addr, &denom))? {
            Some(limit) if !limit.is_zero() => {
                return Err(ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {})
            }
            _ => on_behalf_of_addr,
        }
    } else {
        sender_address.clone()
    };

    let mut market = MARKETS.load(deps.storage, &denom)?;

    if !market.active {
        return Err(ContractError::MarketNotActive {
            denom,
        });
    }

    // Cannot repay zero amount
    if repay_amount.is_zero() {
        return Err(ContractError::InvalidRepayAmount {
            denom,
        });
    }

    // Check new debt
    let debt = DEBTS
        .may_load(deps.storage, (&user_address, &denom))?
        .ok_or(ContractError::CannotRepayZeroDebt {})?;

    let config = CONFIG.load(deps.storage)?;

    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;

    let mut msgs = vec![];
    let mut events = vec![];

    apply_accumulated_interests(
        deps.storage,
        &env,
        &protocol_rewards_collector_address,
        &mut market,
    )?;

    let debt_amount_scaled_before = debt.amount_scaled;
    let debt_amount_before =
        get_underlying_debt_amount(debt.amount_scaled, &market, env.block.time.seconds())?;

    // If repay amount exceeds debt, refund any excess amounts
    let mut refund_amount = Uint128::zero();
    let mut debt_amount_after = Uint128::zero();
    if repay_amount > debt_amount_before {
        refund_amount = repay_amount - debt_amount_before;
        msgs.push(build_send_asset_msg(&user_address, &denom, refund_amount));
    } else {
        debt_amount_after = debt_amount_before - repay_amount;
    }

    // Update the user's debt position
    let debt_amount_scaled_after =
        get_scaled_debt_amount(debt_amount_after, &market, env.block.time.seconds())?;
    let debt_amount_scaled_delta =
        debt_amount_scaled_before.checked_sub(debt_amount_scaled_after)?;
    deduct_debt(deps.storage, &user_address, &denom, debt_amount_scaled_delta, Some(&mut events))?;

    // Update market
    market.debt_total_scaled = market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;
    update_interest_rates(&deps, &env, &mut market, Uint128::zero(), &denom, &mut events)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_events(events)
        .add_attribute("action", "repay")
        .add_attribute("denom", denom)
        .add_attribute("sender", sender_address)
        .add_attribute("user", user_address)
        .add_attribute("amount", repay_amount.checked_sub(refund_amount)?))
}

/// Execute loan liquidations on under-collateralized loans
pub fn liquidate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    liquidator_address: Addr,
    collateral_denom: String,
    debt_denom: String,
    user_address: Addr,
    sent_debt_asset_amount: Uint128,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();

    // 1. Validate liquidation
    // If user (contract) has a positive uncollateralized limit then the user
    // cannot be liquidated
    if let Some(limit) =
        UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&user_address, &debt_denom))?
    {
        if !limit.is_zero() {
            return Err(ContractError::CannotLiquidateWhenPositiveUncollateralizedLoanLimit {});
        }
    };

    let collateral_market = MARKETS.load(deps.storage, &collateral_denom)?;

    if !collateral_market.active {
        return Err(ContractError::MarketNotActive {
            denom: collateral_denom,
        });
    }

    // check if user has collateral in the specified denom available to be liquidated
    let user_collateral = COLLATERALS
        .may_load(deps.storage, (&user_address, &collateral_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoCollateralBalance {})?;
    if !user_collateral.enabled {
        return Err(ContractError::CannotLiquidateWhenCollateralUnset {
            denom: collateral_denom,
        });
    }

    let user_collateral_balance = get_underlying_liquidity_amount(
        user_collateral.amount_scaled,
        &collateral_market,
        block_time,
    )?;
    if user_collateral_balance.is_zero() {
        return Err(ContractError::CannotLiquidateWhenNoCollateralBalance {});
    }

    // check if user has outstanding debt in the deposited asset that needs to be repayed
    let user_debt = DEBTS
        .may_load(deps.storage, (&user_address, &debt_denom))?
        .ok_or(ContractError::CannotLiquidateWhenNoDebtBalance {})?;

    // 2. Compute health factor
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_address = &addresses[&MarsContract::Oracle];

    let user_position =
        get_user_position(deps.as_ref(), block_time, &user_address, oracle_address)?;

    let health_factor = match user_position.health_status {
        // NOTE: Should not get in practice as it would fail on the debt asset check
        UserHealthStatus::NotBorrowing => {
            return Err(ContractError::CannotLiquidateWhenNoDebtBalance {})
        }
        UserHealthStatus::Borrowing(hf) => hf,
    };

    // if health factor is not less than one user cannot be liquidated
    if health_factor >= Decimal::one() {
        return Err(ContractError::CannotLiquidateHealthyPosition {});
    }

    let collateral_and_debt_are_the_same_asset = debt_denom == collateral_denom;

    let debt_market = if !collateral_and_debt_are_the_same_asset {
        MARKETS.load(deps.storage, &debt_denom)?
    } else {
        collateral_market.clone()
    };

    if !debt_market.active {
        return Err(ContractError::MarketNotActive {
            denom: debt_denom,
        });
    }

    // 3. Compute debt to repay and collateral to liquidate
    let collateral_price = user_position.get_asset_price(&collateral_denom)?;
    let debt_price = user_position.get_asset_price(&debt_denom)?;

    let user_debt_asset_total_debt =
        get_underlying_debt_amount(user_debt.amount_scaled, &debt_market, block_time)?;

    let (debt_amount_to_repay, collateral_amount_to_liquidate, refund_amount) =
        liquidation_compute_amounts(
            collateral_price,
            debt_price,
            config.close_factor,
            user_collateral_balance,
            collateral_market.liquidation_bonus,
            user_debt_asset_total_debt,
            sent_debt_asset_amount,
        )?;

    // 4. Update collateral positions and market; transfer collateral shares to the liquidator
    let collateral_amount_to_liquidate_scaled = get_scaled_liquidity_amount(
        collateral_amount_to_liquidate,
        &collateral_market,
        block_time,
    )?;
    deduct_collateral(
        deps.storage,
        &user_address,
        &collateral_denom,
        collateral_amount_to_liquidate_scaled,
        None,
    )?;
    increment_collateral(
        deps.storage,
        &liquidator_address,
        &collateral_denom,
        collateral_amount_to_liquidate_scaled,
        true,
        None,
    )?;

    // 5. Compute and update user new debt
    let user_debt_asset_debt_amount_after =
        user_debt_asset_total_debt.checked_sub(debt_amount_to_repay)?;
    let user_debt_asset_debt_amount_scaled_after = get_scaled_debt_amount(
        user_debt_asset_debt_amount_after,
        &debt_market,
        env.block.time.seconds(),
    )?;
    let debt_amount_scaled_delta =
        user_debt.amount_scaled.checked_sub(user_debt_asset_debt_amount_scaled_after)?;
    deduct_debt(deps.storage, &user_address, &debt_denom, debt_amount_scaled_delta, None)?;

    // 6. Update markets depending on whether the collateral and debt markets are the same
    let mut events = vec![];
    let debt_market_debt_total_scaled_after =
        debt_market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;
    if collateral_and_debt_are_the_same_asset {
        // NOTE: for the sake of clarity copy attributes from collateral market and
        // give generic naming. Debt market could have been used as well
        let mut asset_market_after = collateral_market;
        let denom = &collateral_denom;

        apply_accumulated_interests(
            deps.storage,
            &env,
            protocol_rewards_collector_address,
            &mut asset_market_after,
        )?;

        asset_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        update_interest_rates(
            &deps,
            &env,
            &mut asset_market_after,
            refund_amount,
            denom,
            &mut events,
        )?;

        MARKETS.save(deps.storage, denom, &asset_market_after)?;
    } else {
        let mut debt_market_after = debt_market;

        apply_accumulated_interests(
            deps.storage,
            &env,
            protocol_rewards_collector_address,
            &mut debt_market_after,
        )?;

        debt_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        update_interest_rates(
            &deps,
            &env,
            &mut debt_market_after,
            refund_amount,
            &debt_denom,
            &mut events,
        )?;

        MARKETS.save(deps.storage, &debt_denom, &debt_market_after)?;
    }

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    let mut msgs = vec![];
    if refund_amount > Uint128::zero() {
        msgs.push(build_send_asset_msg(&liquidator_address, &debt_denom, refund_amount));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_events(events)
        .add_attribute("action", "liquidate")
        .add_attribute("collateral_denom", collateral_denom)
        .add_attribute("debt_denom", debt_denom)
        .add_attribute("user", user_address.as_str())
        .add_attribute("liquidator", liquidator_address.as_str())
        .add_attribute("collateral_amount_liquidated", collateral_amount_to_liquidate.to_string())
        .add_attribute("debt_amount_repaid", debt_amount_to_repay.to_string())
        .add_attribute("refund_amount", refund_amount.to_string()))
}

/// Computes debt to repay (in debt asset),
/// collateral to liquidate (in collateral asset) and
/// amount to refund the liquidator (in debt asset)
fn liquidation_compute_amounts(
    collateral_price: Decimal,
    debt_price: Decimal,
    close_factor: Decimal,
    user_collateral_balance: Uint128,
    liquidation_bonus: Decimal,
    user_debt_asset_total_debt: Uint128,
    sent_debt_asset_amount: Uint128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    // Debt: Only up to a fraction of the total debt (determined by the close factor) can be
    // repayed.
    let max_repayable_debt = close_factor * user_debt_asset_total_debt;

    let mut debt_amount_to_repay = if sent_debt_asset_amount > max_repayable_debt {
        max_repayable_debt
    } else {
        sent_debt_asset_amount
    };

    // Collateral: debt to repay in base asset times the liquidation
    // bonus
    let debt_amount_to_repay_in_base_asset = debt_amount_to_repay * debt_price;
    let collateral_amount_to_liquidate_in_base_asset =
        debt_amount_to_repay_in_base_asset * (Decimal::one() + liquidation_bonus);
    let mut collateral_amount_to_liquidate = math::divide_uint128_by_decimal(
        collateral_amount_to_liquidate_in_base_asset,
        collateral_price,
    )?;

    // If collateral amount to liquidate is higher than user_collateral_balance,
    // liquidate the full balance and adjust the debt amount to repay accordingly
    if collateral_amount_to_liquidate > user_collateral_balance {
        collateral_amount_to_liquidate = user_collateral_balance;
        debt_amount_to_repay = math::divide_uint128_by_decimal(
            math::divide_uint128_by_decimal(
                collateral_amount_to_liquidate * collateral_price,
                debt_price,
            )?,
            Decimal::one() + liquidation_bonus,
        )?
    }

    let refund_amount = sent_debt_asset_amount - debt_amount_to_repay;

    Ok((debt_amount_to_repay, collateral_amount_to_liquidate, refund_amount))
}

pub fn update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    enable: bool,
) -> Result<Response, ContractError> {
    let user_address = info.sender;

    let mut events = vec![];

    let mut collateral =
        COLLATERALS.may_load(deps.storage, (&user_address, &denom))?.ok_or_else(|| {
            ContractError::UserNoCollateralBalance {
                user_address: user_address.to_string(),
                denom: denom.clone(),
            }
        })?;

    if !collateral.enabled && enable {
        collateral.enabled = true;
        COLLATERALS.save(deps.storage, (&user_address, &denom), &collateral)?;
        events.push(build_collateral_position_changed_event(
            &denom,
            true,
            user_address.to_string(),
        ));
    } else if collateral.enabled && !enable {
        collateral.enabled = false;
        COLLATERALS.save(deps.storage, (&user_address, &denom), &collateral)?;
        events.push(build_collateral_position_changed_event(
            &denom,
            false,
            user_address.to_string(),
        ));

        // check health factor after disabling collateral
        let config = CONFIG.load(deps.storage)?;
        let oracle_address = address_provider::helpers::query_address(
            deps.as_ref(),
            &config.address_provider_address,
            MarsContract::Oracle,
        )?;
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &user_address,
            &oracle_address,
        )?;
        // if health factor is less than one after disabling collateral we can't process further
        if let UserHealthStatus::Borrowing(health_factor) = user_position.health_status {
            if health_factor < Decimal::one() {
                return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
            }
        }
    }

    Ok(Response::new()
        .add_events(events)
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user_address.as_str())
        .add_attribute("denom", denom)
        .add_attribute("enabled", enable.to_string()))
}
