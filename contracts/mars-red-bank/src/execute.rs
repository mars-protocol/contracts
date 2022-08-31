use std::str;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMarketingInfo;

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::{
    build_send_asset_msg, cw20_get_balance, option_string_to_addr, zero_address,
};
use mars_outpost::red_bank::{
    Config, CreateOrUpdateConfig, Debt, ExecuteMsg, GlobalState, InitOrUpdateAssetParams,
    InstantiateMsg, Market, User,
};
use mars_outpost::{ma_token, math};

use crate::error::ContractError;
use crate::events::{build_collateral_position_changed_event, build_debt_position_changed_event};
use crate::health::{
    assert_below_liq_threshold_after_withdraw, assert_below_max_ltv_after_borrow,
    assert_liquidatable,
};
use crate::helpers::{get_bit, query_total_deposits, set_bit, unset_bit};
use crate::interest_rates::{
    apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
    get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
};
use crate::state::{
    CONFIG, DEBTS, GLOBAL_STATE, MARKETS, MARKET_DENOMS_BY_INDEX, MARKET_DENOMS_BY_MA_TOKEN,
    UNCOLLATERALIZED_LOAN_LIMITS, USERS,
};

pub fn instantiate(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        owner,
        address_provider_address,
        ma_token_code_id,
        close_factor,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && address_provider_address.is_some()
        && ma_token_code_id.is_some()
        && close_factor.is_some();

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
        ma_token_code_id: ma_token_code_id.unwrap(),
        close_factor: close_factor.unwrap(),
    };

    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    GLOBAL_STATE.save(
        deps.storage,
        &GlobalState {
            market_count: 0,
        },
    )?;

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
        ma_token_code_id,
        close_factor,
    } = new_config;

    // Update config
    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address =
        option_string_to_addr(deps.api, address_provider_address, config.address_provider_address)?;
    config.ma_token_code_id = ma_token_code_id.unwrap_or(config.ma_token_code_id);
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
    asset_symbol_option: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let mut money_market = GLOBAL_STATE.load(deps.storage)?;

    let market_option = MARKETS.may_load(deps.storage, &denom)?;
    match market_option {
        None => {
            let market_idx = money_market.market_count;
            let new_market =
                create_market(env.block.time.seconds(), market_idx, &denom, asset_params)?;

            // Save new market
            MARKETS.save(deps.storage, &denom, &new_market)?;

            // Save index to reference mapping
            MARKET_DENOMS_BY_INDEX.save(deps.storage, market_idx, &denom)?;

            // Increment market count
            money_market.market_count += 1;
            GLOBAL_STATE.save(deps.storage, &money_market)?;

            let symbol = asset_symbol_option.unwrap_or_else(|| denom.clone());

            // Prepare response, should instantiate an maToken
            // and use the Register hook.
            // A new maToken should be created which callbacks this contract in order to be registered.
            let addresses = address_provider::helpers::query_addresses(
                deps.as_ref(),
                &config.address_provider_address,
                vec![MarsContract::Incentives, MarsContract::ProtocolAdmin],
            )?;
            // TODO: protocol admin may be a marshub address, which can't be validated into `Addr`
            let protocol_admin_address = &addresses[&MarsContract::ProtocolAdmin];
            let incentives_address = &addresses[&MarsContract::Incentives];

            let token_symbol = format!("ma{}", symbol);

            let res = Response::new()
                .add_attribute("action", "init_asset")
                .add_attribute("denom", &denom)
                .add_message(CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: Some(protocol_admin_address.to_string()),
                    code_id: config.ma_token_code_id,
                    msg: to_binary(&ma_token::msg::InstantiateMsg {
                        name: format!("Mars {} Liquidity Token", symbol),
                        symbol: token_symbol.clone(),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: env.contract.address.to_string(),
                            cap: None,
                        }),
                        marketing: Some(InstantiateMarketingInfo {
                            project: Some(String::from("Mars Protocol")),
                            description: Some(format!(
                                "Interest earning token representing deposits for {}",
                                symbol
                            )),
                            marketing: Some(protocol_admin_address.to_string()),
                            logo: None,
                        }),
                        init_hook: Some(ma_token::msg::InitHook {
                            contract_addr: env.contract.address.to_string(),
                            msg: to_binary(&ExecuteMsg::InitAssetTokenCallback {
                                denom,
                            })?,
                        }),
                        red_bank_address: env.contract.address.to_string(),
                        incentives_address: incentives_address.into(),
                    })?,
                    funds: vec![],
                    label: token_symbol,
                }));
            Ok(res)
        }
        Some(_) => Err(ContractError::AssetAlreadyInitialized {}),
    }
}

/// Initialize new market
pub fn create_market(
    block_time: u64,
    index: u32,
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
        interest_rate_model,
        deposit_enabled,
        borrow_enabled,
        deposit_cap,
    } = params;

    // All fields should be available
    let available = borrow_rate.is_some()
        && max_loan_to_value.is_some()
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
        index,
        denom: denom.to_string(),
        ma_token_address: Addr::unchecked(""),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: borrow_rate.unwrap(),
        liquidity_rate: Decimal::zero(),
        max_loan_to_value: max_loan_to_value.unwrap(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
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

pub fn init_asset_token_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    let mut market = MARKETS.load(deps.storage, &denom)?;

    if market.ma_token_address == zero_address() {
        let ma_contract_addr = info.sender;

        market.ma_token_address = ma_contract_addr.clone();
        MARKETS.save(deps.storage, &denom, &market)?;

        // save ma token contract to reference mapping
        MARKET_DENOMS_BY_MA_TOKEN.save(deps.storage, &ma_contract_addr, &denom)?;

        let res = Response::new().add_attribute("action", "init_asset_token_callback");
        Ok(res)
    } else {
        // Can do this only once
        Err(MarsError::Unauthorized {}.into())
    }
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
                interest_rate_model,
                deposit_enabled,
                borrow_enabled,
                deposit_cap,
            } = asset_params;

            // If reserve factor or interest rates are updated we update indexes with
            // current values before applying the change to prevent applying this
            // new params to a period where they were not valid yet. Interests rates are
            // recalculated after changes are applied.
            let should_update_interest_rates = (reserve_factor.is_some()
                && reserve_factor.unwrap() != market.reserve_factor)
                || interest_rate_model.is_some();

            let mut response = Response::new();

            if should_update_interest_rates {
                let protocol_rewards_collector_address = address_provider::helpers::query_address(
                    deps.as_ref(),
                    &config.address_provider_address,
                    MarsContract::ProtocolRewardsCollector,
                )?;
                response = apply_accumulated_interests(
                    &env,
                    &protocol_rewards_collector_address,
                    &mut market,
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
                response = update_interest_rates(
                    &deps,
                    &env,
                    &mut updated_market,
                    Uint128::zero(),
                    &denom,
                    response,
                )?;
            }
            MARKETS.save(deps.storage, &denom, &updated_market)?;

            response =
                response.add_attribute("action", "update_asset").add_attribute("denom", &denom);

            Ok(response)
        }
    }
}

/// Update uncollateralized loan limit by a given amount in base asset
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
    if let Some(user) = USERS.may_load(deps.storage, &user_address)? {
        let previous_uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
            .may_load(deps.storage, (&denom, &user_address))?
            .unwrap_or_else(Uint128::zero);

        if previous_uncollateralized_loan_limit == Uint128::zero() {
            let asset_market = MARKETS.load(deps.storage, &denom)?;

            let is_borrowing_asset = get_bit(user.borrowed_assets, asset_market.index)?;

            if is_borrowing_asset {
                return Err(ContractError::UserHasCollateralizedDebt {});
            }
        };
    }

    UNCOLLATERALIZED_LOAN_LIMITS.save(deps.storage, (&denom, &user_address), &new_limit)?;

    DEBTS.update(
        deps.storage,
        (&denom, &user_address),
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

    let res = Response::new()
        .add_attribute("action", "update_uncollateralized_loan_limit")
        .add_attribute("user", user_address.as_str())
        .add_attribute("denom", denom)
        .add_attribute("new_allowance", new_limit.to_string());
    Ok(res)
}

/// Execute deposits and mint corresponding ma_tokens
pub fn deposit(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender_addr: Addr,
    on_behalf_of: Option<String>,
    denom: String,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_addr = if let Some(address) = on_behalf_of {
        deps.api.addr_validate(&address)?
    } else {
        sender_addr.clone()
    };

    let mut market = MARKETS.load(deps.storage, &denom)?;
    if !market.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            denom,
        });
    }

    let total_scaled_deposits = query_total_deposits(&deps.querier, &market.ma_token_address)?;
    let total_deposits =
        get_underlying_liquidity_amount(total_scaled_deposits, &market, env.block.time.seconds())?;
    if total_deposits.checked_add(deposit_amount)? > market.deposit_cap {
        return Err(ContractError::DepositCapExceeded {
            denom,
        });
    }

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(ContractError::InvalidDepositAmount {
            denom,
        });
    }

    let mut user = USERS.may_load(deps.storage, &user_addr)?.unwrap_or_default();

    let mut response = Response::new();
    let has_deposited_asset = get_bit(user.collateral_assets, market.index)?;
    if !has_deposited_asset {
        set_bit(&mut user.collateral_assets, market.index)?;
        USERS.save(deps.storage, &user_addr, &user)?;
        response = response.add_event(build_collateral_position_changed_event(
            &denom,
            true,
            user_addr.to_string(),
        ));
    }

    let config = CONFIG.load(deps.storage)?;

    // update indexes and interest rates
    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;
    response = apply_accumulated_interests(
        &env,
        &protocol_rewards_collector_address,
        &mut market,
        response,
    )?;
    response = update_interest_rates(&deps, &env, &mut market, Uint128::zero(), &denom, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }
    let mint_amount =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;

    response = response
        .add_attribute("action", "deposit")
        .add_attribute("denom", denom)
        .add_attribute("sender", sender_addr)
        .add_attribute("user", user_addr.as_str())
        .add_attribute("amount", deposit_amount)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market.ma_token_address.into(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: user_addr.into(),
                amount: mint_amount,
            })?,
            funds: vec![],
        }));

    Ok(response)
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

    let asset_ma_addr = market.ma_token_address.clone();
    let withdrawer_balance_scaled_before =
        cw20_get_balance(&deps.querier, asset_ma_addr, withdrawer_addr.clone())?;
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
    let oracle_addr = &addresses[&MarsContract::Oracle];

    let mut withdrawer = match USERS.may_load(deps.storage, &withdrawer_addr)? {
        Some(user) => user,
        None => {
            // No address should withdraw without an existing user position already in
            // storage (If this happens the protocol did something wrong). The exception is
            // the protocol_rewards_collector which gets minted token without depositing
            // nor receiving a transfer from another user.
            if withdrawer_addr != *protocol_rewards_collector_address {
                return Err(ContractError::ExistingUserPositionRequired {});
            }
            User::default()
        }
    };
    let asset_as_collateral = get_bit(withdrawer.collateral_assets, market.index)?;
    let user_is_borrowing = !withdrawer.borrowed_assets.is_zero();

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if asset_as_collateral
        && user_is_borrowing
        && !assert_below_liq_threshold_after_withdraw(
            &deps.as_ref(),
            &env,
            &withdrawer,
            &withdrawer_addr,
            oracle_addr,
            &denom,
            withdraw_amount,
        )?
    {
        return Err(ContractError::InvalidHealthFactorAfterWithdraw {});
    }

    let mut response = Response::new();

    // if amount to withdraw equals the user's balance then unset collateral bit
    if asset_as_collateral && withdraw_amount == withdrawer_balance_before {
        unset_bit(&mut withdrawer.collateral_assets, market.index)?;
        USERS.save(deps.storage, &withdrawer_addr, &withdrawer)?;
        response = response.add_event(build_collateral_position_changed_event(
            &denom,
            false,
            withdrawer_addr.to_string(),
        ));
    }

    // update indexes and interest rates
    response = apply_accumulated_interests(
        &env,
        protocol_rewards_collector_address,
        &mut market,
        response,
    )?;
    response = update_interest_rates(&deps, &env, &mut market, withdraw_amount, &denom, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    // burn maToken
    let withdrawer_balance_after = withdrawer_balance_before.checked_sub(withdraw_amount)?;
    let withdrawer_balance_scaled_after =
        get_scaled_liquidity_amount(withdrawer_balance_after, &market, env.block.time.seconds())?;

    let burn_amount =
        withdrawer_balance_scaled_before.checked_sub(withdrawer_balance_scaled_after)?;
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: market.ma_token_address.to_string(),
        msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
            user: withdrawer_addr.to_string(),
            amount: burn_amount,
        })?,
        funds: vec![],
    }));

    // send underlying asset to user or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        withdrawer_addr.clone()
    };
    response =
        response.add_message(build_send_asset_msg(&recipient_address, &denom, withdraw_amount));

    response = response
        .add_attribute("action", "withdraw")
        .add_attribute("denom", denom)
        .add_attribute("user", withdrawer_addr.as_str())
        .add_attribute("recipient", recipient_address.as_str())
        .add_attribute("burn_amount", burn_amount)
        .add_attribute("withdraw_amount", withdraw_amount);
    Ok(response)
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
    let borrower_addr = info.sender;

    // Cannot borrow zero amount
    if borrow_amount.is_zero() {
        return Err(ContractError::InvalidBorrowAmount {
            denom,
        });
    }

    // Load market and user state
    let mut borrow_market = MARKETS.load(deps.storage, &denom)?;

    if !borrow_market.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            denom,
        });
    }

    let uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (&denom, &borrower_addr))?
        .unwrap_or_else(Uint128::zero);
    let mut user: User = match USERS.may_load(deps.storage, &borrower_addr)? {
        Some(user) => user,
        None => {
            if uncollateralized_loan_limit.is_zero() {
                return Err(ContractError::UserNoCollateral {});
            }
            // If User has some uncollateralized_loan_limit, then we don't require an existing debt position and initialize a new one.
            User::default()
        }
    };

    let is_borrowing_asset = get_bit(user.borrowed_assets, borrow_market.index)?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_addr = &addresses[&MarsContract::Oracle];

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if uncollateralized_loan_limit.is_zero() {
        if !assert_below_max_ltv_after_borrow(
            &deps.as_ref(),
            &env,
            &user,
            &borrower_addr,
            oracle_addr,
            &denom,
            borrow_amount,
        )? {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    } else {
        // Uncollateralized loan: check borrow amount plus debt does not exceed uncollateralized loan limit
        uncollateralized_debt = true;

        let borrower_debt =
            DEBTS.may_load(deps.storage, (&denom, &borrower_addr))?.unwrap_or(Debt {
                amount_scaled: Uint128::zero(),
                uncollateralized: uncollateralized_debt,
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

    let mut response = Response::new();

    response = apply_accumulated_interests(
        &env,
        protocol_rewards_collector_address,
        &mut borrow_market,
        response,
    )?;

    // Set borrowing asset for user
    if !is_borrowing_asset {
        set_bit(&mut user.borrowed_assets, borrow_market.index)?;
        USERS.save(deps.storage, &borrower_addr, &user)?;
        response = response.add_event(build_debt_position_changed_event(
            &denom,
            true,
            borrower_addr.to_string(),
        ));
    }

    // Set new debt
    let mut debt = DEBTS.may_load(deps.storage, (&denom, &borrower_addr))?.unwrap_or(Debt {
        amount_scaled: Uint128::zero(),
        uncollateralized: uncollateralized_debt,
    });
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;
    debt.amount_scaled = debt.amount_scaled.checked_add(borrow_amount_scaled)?;
    DEBTS.save(deps.storage, (&denom, &borrower_addr), &debt)?;

    borrow_market.debt_total_scaled += borrow_amount_scaled;

    response =
        update_interest_rates(&deps, &env, &mut borrow_market, borrow_amount, &denom, response)?;
    MARKETS.save(deps.storage, &denom, &borrow_market)?;

    // Send borrow amount to borrower or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        borrower_addr.clone()
    };
    response =
        response.add_message(build_send_asset_msg(&recipient_address, &denom, borrow_amount));

    response = response
        .add_attribute("action", "borrow")
        .add_attribute("denom", denom)
        .add_attribute("user", borrower_addr.as_str())
        .add_attribute("recipient", recipient_address.as_str())
        .add_attribute("amount", borrow_amount);
    Ok(response)
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
        match UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&denom, &on_behalf_of_addr))? {
            Some(limit) if !limit.is_zero() => {
                return Err(ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {})
            }
            _ => on_behalf_of_addr,
        }
    } else {
        sender_address.clone()
    };

    // Cannot repay zero amount
    if repay_amount.is_zero() {
        return Err(ContractError::InvalidRepayAmount {
            denom,
        });
    }

    // Check new debt
    let mut debt = DEBTS.load(deps.storage, (&denom, &user_address))?;

    if debt.amount_scaled.is_zero() {
        return Err(ContractError::CannotRepayZeroDebt {});
    }

    let config = CONFIG.load(deps.storage)?;

    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;

    let mut market = MARKETS.load(deps.storage, &denom)?;

    let mut response = Response::new();

    response = apply_accumulated_interests(
        &env,
        &protocol_rewards_collector_address,
        &mut market,
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
        let refund_msg = build_send_asset_msg(&user_address, &denom, refund_amount);
        response = response.add_message(refund_msg);
    } else {
        debt_amount_after = debt_amount_before - repay_amount;
    }

    let debt_amount_scaled_after =
        get_scaled_debt_amount(debt_amount_after, &market, env.block.time.seconds())?;
    debt.amount_scaled = debt_amount_scaled_after;
    DEBTS.save(deps.storage, (&denom, &user_address), &debt)?;

    let debt_amount_scaled_delta =
        debt_amount_scaled_before.checked_sub(debt_amount_scaled_after)?;

    market.debt_total_scaled = market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

    response = update_interest_rates(&deps, &env, &mut market, Uint128::zero(), &denom, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    if debt.amount_scaled.is_zero() {
        // Remove asset from borrowed assets
        let mut user = USERS.load(deps.storage, &user_address)?;
        unset_bit(&mut user.borrowed_assets, market.index)?;
        USERS.save(deps.storage, &user_address, &user)?;
        response = response.add_event(build_debt_position_changed_event(
            &denom,
            false,
            user_address.to_string(),
        ));
    }

    response = response
        .add_attribute("action", "repay")
        .add_attribute("denom", denom)
        .add_attribute("sender", sender_address)
        .add_attribute("user", user_address)
        .add_attribute("amount", repay_amount.checked_sub(refund_amount)?);
    Ok(response)
}

/// Execute loan liquidations on under-collateralized loans
pub fn liquidate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    liquidator_address: Addr,
    collateral_denom: String,
    debt_denom: String,
    user_addr: Addr,
    sent_debt_asset_amount: Uint128,
    receive_ma_token: bool,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();

    // 1. Validate liquidation
    // If user (contract) has a positive uncollateralized limit then the user
    // cannot be liquidated
    if let Some(limit) =
        UNCOLLATERALIZED_LOAN_LIMITS.may_load(deps.storage, (&debt_denom, &user_addr))?
    {
        if !limit.is_zero() {
            return Err(ContractError::CannotLiquidateWhenPositiveUncollateralizedLoanLimit {});
        }
    };

    let mut user = USERS.load(deps.storage, &user_addr)?;
    let collateral_market = MARKETS.load(deps.storage, &collateral_denom)?;
    let using_collateral_asset_as_collateral =
        get_bit(user.collateral_assets, collateral_market.index)?;
    if !using_collateral_asset_as_collateral {
        return Err(ContractError::CannotLiquidateWhenCollateralUnset {
            denom: collateral_denom,
        });
    }

    // check if user has available collateral in specified collateral asset to be liquidated
    let user_collateral_balance_scaled = cw20_get_balance(
        &deps.querier,
        collateral_market.ma_token_address.clone(),
        user_addr.clone(),
    )?;
    let user_collateral_balance = get_underlying_liquidity_amount(
        user_collateral_balance_scaled,
        &collateral_market,
        block_time,
    )?;
    if user_collateral_balance.is_zero() {
        return Err(ContractError::CannotLiquidateWhenNoCollateralBalance {});
    }

    // check if user has outstanding debt in the deposited asset that needs to be repayed
    let mut user_debt = DEBTS.load(deps.storage, (&debt_denom, &user_addr))?;
    if user_debt.amount_scaled.is_zero() {
        return Err(ContractError::CannotLiquidateWhenNoDebtBalance {});
    }

    // 2. Compute health factor
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_addr = &addresses[&MarsContract::Oracle];

    let (liquidatable, assets_positions) =
        assert_liquidatable(&deps.as_ref(), &env, &user, &user_addr, oracle_addr)?;

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

    // 4. Update collateral positions and market depending on whether the liquidator elects to
    // receive ma_tokens or the underlying asset
    if receive_ma_token {
        response = process_ma_token_transfer_to_liquidator(
            deps.branch(),
            block_time,
            &user_addr,
            &liquidator_address,
            &collateral_denom,
            &collateral_market,
            collateral_amount_to_liquidate,
            response,
        )?;
    } else {
        response = process_underlying_asset_transfer_to_liquidator(
            deps.branch(),
            &env,
            &user_addr,
            &liquidator_address,
            &collateral_denom,
            &collateral_market,
            collateral_amount_to_liquidate,
            response,
        )?;
    }

    // if max collateral to liquidate equals the user's balance then unset collateral bit
    if collateral_amount_to_liquidate == user_collateral_balance {
        unset_bit(&mut user.collateral_assets, collateral_market.index)?;
        USERS.save(deps.storage, &user_addr, &user)?;
        response = response.add_event(build_collateral_position_changed_event(
            &collateral_denom,
            false,
            user_addr.to_string(),
        ));
    }

    // 5. Compute and update user new debt
    let user_debt_asset_debt_amount_after =
        user_debt_asset_total_debt.checked_sub(debt_amount_to_repay)?;
    let user_debt_asset_debt_amount_scaled_after = get_scaled_debt_amount(
        user_debt_asset_debt_amount_after,
        &debt_market,
        env.block.time.seconds(),
    )?;

    // Compute delta so it can be substracted to total debt
    let debt_amount_scaled_delta =
        user_debt.amount_scaled.checked_sub(user_debt_asset_debt_amount_scaled_after)?;

    user_debt.amount_scaled = user_debt_asset_debt_amount_scaled_after;

    DEBTS.save(deps.storage, (&debt_denom, &user_addr), &user_debt)?;

    let debt_market_debt_total_scaled_after =
        debt_market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

    // 6. Update markets depending on whether the collateral and debt markets are the same
    // and whether the liquidator receives ma_tokens (no change in liquidity) or underlying asset
    // (changes liquidity)
    if collateral_and_debt_are_the_same_asset {
        // NOTE: for the sake of clarity copy attributes from collateral market and
        // give generic naming. Debt market could have been used as well
        let mut asset_market_after = collateral_market;
        let denom = &collateral_denom;

        response = apply_accumulated_interests(
            &env,
            protocol_rewards_collector_address,
            &mut asset_market_after,
            response,
        )?;

        asset_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        let mut less_liquidity = refund_amount;

        if !receive_ma_token {
            less_liquidity = less_liquidity.checked_add(collateral_amount_to_liquidate)?;
        };

        response = update_interest_rates(
            &deps,
            &env,
            &mut asset_market_after,
            less_liquidity,
            denom,
            response,
        )?;

        MARKETS.save(deps.storage, denom, &asset_market_after)?;
    } else {
        if !receive_ma_token {
            let mut collateral_market_after = collateral_market;

            response = apply_accumulated_interests(
                &env,
                protocol_rewards_collector_address,
                &mut collateral_market_after,
                response,
            )?;

            response = update_interest_rates(
                &deps,
                &env,
                &mut collateral_market_after,
                collateral_amount_to_liquidate,
                &collateral_denom,
                response,
            )?;

            MARKETS.save(deps.storage, &collateral_denom, &collateral_market_after)?;
        }

        let mut debt_market_after = debt_market;

        response = apply_accumulated_interests(
            &env,
            protocol_rewards_collector_address,
            &mut debt_market_after,
            response,
        )?;

        debt_market_after.debt_total_scaled = debt_market_debt_total_scaled_after;

        response = update_interest_rates(
            &deps,
            &env,
            &mut debt_market_after,
            refund_amount,
            &debt_denom,
            response,
        )?;

        MARKETS.save(deps.storage, &debt_denom, &debt_market_after)?;
    }

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    if refund_amount > Uint128::zero() {
        response = response.add_message(build_send_asset_msg(
            &liquidator_address,
            &debt_denom,
            refund_amount,
        ));
    }

    response = response
        .add_attribute("action", "liquidate")
        .add_attribute("collateral_denom", collateral_denom)
        .add_attribute("debt_denom", debt_denom)
        .add_attribute("user", user_addr.as_str())
        .add_attribute("liquidator", liquidator_address.as_str())
        .add_attribute("collateral_amount_liquidated", collateral_amount_to_liquidate.to_string())
        .add_attribute("debt_amount_repaid", debt_amount_to_repay.to_string())
        .add_attribute("refund_amount", refund_amount.to_string());
    Ok(response)
}

/// Transfer ma tokens from user to liquidator
/// Returns response with added messages and events
fn process_ma_token_transfer_to_liquidator(
    deps: DepsMut,
    block_time: u64,
    user_addr: &Addr,
    liquidator_addr: &Addr,
    collateral_denom: &str,
    collateral_market: &Market,
    collateral_amount_to_liquidate: Uint128,
    mut response: Response,
) -> StdResult<Response> {
    let mut liquidator = USERS.may_load(deps.storage, liquidator_addr)?.unwrap_or_default();

    // Set liquidator's deposited bit to true if not already true
    // NOTE: previous checks should ensure amount to be sent is not zero
    let liquidator_is_using_as_collateral =
        get_bit(liquidator.collateral_assets, collateral_market.index)?;
    if !liquidator_is_using_as_collateral {
        set_bit(&mut liquidator.collateral_assets, collateral_market.index)?;
        USERS.save(deps.storage, liquidator_addr, &liquidator)?;
        response = response.add_event(build_collateral_position_changed_event(
            collateral_denom,
            true,
            liquidator_addr.to_string(),
        ));
    }

    let collateral_amount_to_liquidate_scaled =
        get_scaled_liquidity_amount(collateral_amount_to_liquidate, collateral_market, block_time)?;

    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collateral_market.ma_token_address.to_string(),
        msg: to_binary(&mars_outpost::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
            sender: user_addr.to_string(),
            recipient: liquidator_addr.to_string(),
            amount: collateral_amount_to_liquidate_scaled,
        })?,
        funds: vec![],
    }));

    Ok(response)
}

/// Burn ma_tokens from user and send underlying asset to liquidator
/// Returns response with added messages and events
pub fn process_underlying_asset_transfer_to_liquidator(
    deps: DepsMut,
    env: &Env,
    user_addr: &Addr,
    liquidator_addr: &Addr,
    collateral_denom: &str,
    collateral_market: &Market,
    collateral_amount_to_liquidate: Uint128,
    mut response: Response,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();

    // Ensure contract has enough collateral to send back underlying asset
    let contract_collateral_balance =
        deps.querier.query_balance(&env.contract.address, collateral_denom)?.amount;

    if contract_collateral_balance < collateral_amount_to_liquidate {
        return Err(ContractError::CannotLiquidateWhenNotEnoughCollateral {});
    }

    let collateral_amount_to_liquidate_scaled =
        get_scaled_liquidity_amount(collateral_amount_to_liquidate, collateral_market, block_time)?;

    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collateral_market.ma_token_address.to_string(),
        msg: to_binary(&mars_outpost::ma_token::msg::ExecuteMsg::Burn {
            user: user_addr.to_string(),

            amount: collateral_amount_to_liquidate_scaled,
        })?,
        funds: vec![],
    }));

    response = response.add_message(build_send_asset_msg(
        liquidator_addr,
        collateral_denom,
        collateral_amount_to_liquidate,
    ));

    Ok(response)
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

/// Update (enable / disable) collateral asset for specific user
pub fn update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    enable: bool,
) -> Result<Response, ContractError> {
    let user_addr = info.sender;
    let mut user = USERS.may_load(deps.storage, &user_addr)?.unwrap_or_default();

    let mut events = vec![];

    let collateral_market = MARKETS.load(deps.storage, &denom)?;
    let has_collateral_asset = get_bit(user.collateral_assets, collateral_market.index)?;
    if !has_collateral_asset && enable {
        let collateral_ma_address = collateral_market.ma_token_address;
        let user_collateral_balance =
            cw20_get_balance(&deps.querier, collateral_ma_address, user_addr.clone())?;
        if user_collateral_balance > Uint128::zero() {
            // enable collateral asset
            set_bit(&mut user.collateral_assets, collateral_market.index)?;
            USERS.save(deps.storage, &user_addr, &user)?;
            events.push(build_collateral_position_changed_event(
                &denom,
                true,
                user_addr.to_string(),
            ));
        } else {
            return Err(ContractError::UserNoCollateralBalance {
                user_address: user_addr.to_string(),
                denom,
            });
        }
    } else if has_collateral_asset && !enable {
        // disable collateral asset
        unset_bit(&mut user.collateral_assets, collateral_market.index)?;

        // check health factor after disabling collateral
        let config = CONFIG.load(deps.storage)?;
        let oracle_addr = address_provider::helpers::query_address(
            deps.as_ref(),
            &config.address_provider_address,
            MarsContract::Oracle,
        )?;

        let (liquidatable, _) =
            assert_liquidatable(&deps.as_ref(), &env, &user, &user_addr, &oracle_addr)?;

        if liquidatable {
            return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
        }

        USERS.save(deps.storage, &user_addr, &user)?;
        events.push(build_collateral_position_changed_event(&denom, false, user_addr.to_string()));
    }

    let res = Response::new()
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user_addr.as_str())
        .add_attribute("denom", denom)
        .add_attribute("has_collateral", has_collateral_asset.to_string())
        .add_attribute("enable", enable.to_string())
        .add_events(events);
    Ok(res)
}

/// Update uncollateralized loan limit by a given amount in base asset
pub fn finalize_liquidity_token_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    from_address: Addr,
    to_address: Addr,
    from_previous_balance: Uint128,
    to_previous_balance: Uint128,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Get liquidity token market
    let denom = MARKET_DENOMS_BY_MA_TOKEN.load(deps.storage, &info.sender)?;
    let market = MARKETS.load(deps.storage, &denom)?;

    // Check user health factor is above 1
    let mut from_user = USERS.load(deps.storage, &from_address)?;
    let config = CONFIG.load(deps.storage)?;
    let oracle_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::Oracle,
    )?;

    let (liquidatable, _) =
        assert_liquidatable(&deps.as_ref(), &env, &from_user, &from_address, &oracle_address)?;

    if liquidatable {
        return Err(ContractError::CannotTransferTokenWhenInvalidHealthFactor {});
    }

    let mut events = vec![];

    // Update users's positions
    if from_address != to_address {
        if from_previous_balance.checked_sub(amount)?.is_zero() {
            unset_bit(&mut from_user.collateral_assets, market.index)?;
            USERS.save(deps.storage, &from_address, &from_user)?;
            events.push(build_collateral_position_changed_event(
                &denom,
                false,
                from_address.to_string(),
            ))
        }

        if to_previous_balance.is_zero() && !amount.is_zero() {
            let mut to_user = USERS.may_load(deps.storage, &to_address)?.unwrap_or_default();
            set_bit(&mut to_user.collateral_assets, market.index)?;
            USERS.save(deps.storage, &to_address, &to_user)?;
            events.push(build_collateral_position_changed_event(
                &denom,
                true,
                to_address.to_string(),
            ))
        }
    }

    let res = Response::new()
        .add_attribute("action", "finalize_liquidity_token_transfer")
        .add_events(events);
    Ok(res)
}
