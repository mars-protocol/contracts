use std::str;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use cw20_base::msg::InstantiateMarketingInfo;
use cw_storage_plus::U32Key;

use mars_core::address_provider::{self, MarsContract};
use mars_core::ma_token;

use mars_core::asset::{
    build_send_asset_with_tax_deduction_msg, get_asset_balance, Asset, AssetType,
};
use mars_core::error::MarsError;
use mars_core::helpers::{cw20_get_balance, cw20_get_symbol, option_string_to_addr, zero_address};
use mars_core::math::decimal::Decimal;

use crate::accounts::get_user_position;
use crate::error::ContractError;
use crate::error::ContractError::InvalidNativeCoinsSent;
use crate::interest_rate_models::init_interest_rate_model;
use crate::interest_rates::{
    apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
    get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
};
use crate::msg::{
    CreateOrUpdateConfig, ExecuteMsg, InitOrUpdateAssetParams, InstantiateMsg, QueryMsg, ReceiveMsg,
};
use crate::state::{
    CONFIG, DEBTS, GLOBAL_STATE, MARKETS, MARKET_REFERENCES_BY_INDEX,
    MARKET_REFERENCES_BY_MA_TOKEN, UNCOLLATERALIZED_LOAN_LIMITS, USERS,
};
use crate::{
    Config, ConfigResponse, Debt, GlobalState, Market, MarketInfo, MarketsListResponse, User,
    UserAssetCollateralResponse, UserAssetDebtResponse, UserCollateralResponse, UserDebtResponse,
    UserHealthStatus, UserPositionResponse,
};

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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

    GLOBAL_STATE.save(deps.storage, &GlobalState { market_count: 0 })?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => execute_receive_cw20(deps, env, info, cw20_msg),

        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, env, info, config),

        ExecuteMsg::InitAsset {
            asset,
            asset_params,
            asset_symbol,
        } => execute_init_asset(deps, env, info, asset, asset_params, asset_symbol),

        ExecuteMsg::InitAssetTokenCallback { reference } => {
            execute_init_asset_token_callback(deps, env, info, reference)
        }

        ExecuteMsg::UpdateAsset {
            asset,
            asset_params,
        } => execute_update_asset(deps, env, info, asset, asset_params),

        ExecuteMsg::UpdateUncollateralizedLoanLimit {
            user_address,
            asset,
            new_limit,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            execute_update_uncollateralized_loan_limit(deps, env, info, user_addr, asset, new_limit)
        }

        ExecuteMsg::DepositNative {
            denom,
            on_behalf_of,
        } => {
            let deposit_amount = get_denom_amount_from_coins(&info.funds, &denom)?;
            let depositor_address = info.sender.clone();
            execute_deposit(
                deps,
                env,
                info,
                depositor_address,
                on_behalf_of,
                denom.as_bytes(),
                denom.as_str(),
                deposit_amount,
            )
        }

        ExecuteMsg::Withdraw {
            asset,
            amount,
            recipient: recipient_address,
        } => execute_withdraw(deps, env, info, asset, amount, recipient_address),
        ExecuteMsg::Borrow {
            asset,
            amount,
            recipient: recipient_address,
        } => execute_borrow(deps, env, info, asset, amount, recipient_address),

        ExecuteMsg::RepayNative {
            denom,
            on_behalf_of,
        } => {
            let repayer_address = info.sender.clone();
            let repay_amount = get_denom_amount_from_coins(&info.funds, &denom)?;

            execute_repay(
                deps,
                env,
                info,
                repayer_address,
                on_behalf_of,
                denom.as_bytes(),
                denom.clone(),
                repay_amount,
                AssetType::Native,
            )
        }

        ExecuteMsg::LiquidateNative {
            collateral_asset,
            debt_asset_denom,
            user_address,
            receive_ma_token,
        } => {
            let sender = info.sender.clone();
            let user_addr = deps.api.addr_validate(&user_address)?;
            let sent_debt_asset_amount =
                get_denom_amount_from_coins(&info.funds, &debt_asset_denom)?;
            execute_liquidate(
                deps,
                env,
                info,
                sender,
                collateral_asset,
                Asset::Native {
                    denom: debt_asset_denom,
                },
                user_addr,
                sent_debt_asset_amount,
                receive_ma_token,
            )
        }

        ExecuteMsg::UpdateAssetCollateralStatus { asset, enable } => {
            execute_update_asset_collateral_status(deps, env, info, asset, enable)
        }

        ExecuteMsg::FinalizeLiquidityTokenTransfer {
            sender_address,
            recipient_address,
            sender_previous_balance,
            recipient_previous_balance,
            amount,
        } => execute_finalize_liquidity_token_transfer(
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

/// cw20 receive implementation
pub fn execute_receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::DepositCw20 { on_behalf_of } => {
            let depositor_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            let token_contract_address = info.sender.clone();
            execute_deposit(
                deps,
                env,
                info,
                depositor_addr,
                on_behalf_of,
                token_contract_address.as_bytes(),
                token_contract_address.as_str(),
                cw20_msg.amount,
            )
        }
        ReceiveMsg::RepayCw20 { on_behalf_of } => {
            let repayer_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            let token_contract_address = info.sender.clone();
            execute_repay(
                deps,
                env,
                info,
                repayer_addr,
                on_behalf_of,
                token_contract_address.as_bytes(),
                token_contract_address.to_string(),
                cw20_msg.amount,
                AssetType::Cw20,
            )
        }
        ReceiveMsg::LiquidateCw20 {
            collateral_asset,
            user_address,
            receive_ma_token,
        } => {
            let debt_asset_addr = info.sender.clone();
            let liquidator_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            let user_addr = deps.api.addr_validate(&user_address)?;
            execute_liquidate(
                deps,
                env,
                info,
                liquidator_addr,
                collateral_asset,
                Asset::Cw20 {
                    contract_addr: debt_asset_addr.to_string(),
                },
                user_addr,
                cw20_msg.amount,
                receive_ma_token,
            )
        }
    }
}

/// Update config
pub fn execute_update_config(
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
    config.address_provider_address = option_string_to_addr(
        deps.api,
        address_provider_address,
        config.address_provider_address,
    )?;
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
pub fn execute_init_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    asset_params: InitOrUpdateAssetParams,
    asset_symbol_option: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let mut money_market = GLOBAL_STATE.load(deps.storage)?;

    let (asset_label, asset_reference, asset_type) = asset.get_attributes();
    let market_option = MARKETS.may_load(deps.storage, asset_reference.as_slice())?;
    match market_option {
        None => {
            let market_idx = money_market.market_count;
            let new_market = create_market(
                env.block.time.seconds(),
                market_idx,
                asset_type,
                asset_params,
            )?;

            // Save new market
            MARKETS.save(deps.storage, asset_reference.as_slice(), &new_market)?;

            // Save index to reference mapping
            MARKET_REFERENCES_BY_INDEX.save(
                deps.storage,
                U32Key::new(market_idx),
                &asset_reference.to_vec(),
            )?;

            // Increment market count
            money_market.market_count += 1;
            GLOBAL_STATE.save(deps.storage, &money_market)?;

            let symbol = if let Some(asset_symbol) = asset_symbol_option {
                asset_symbol
            } else {
                match asset {
                    Asset::Native { denom } => denom,
                    Asset::Cw20 { contract_addr } => {
                        let contract_addr = deps.api.addr_validate(&contract_addr)?;
                        cw20_get_symbol(&deps.querier, contract_addr)?
                    }
                }
            };

            // Prepare response, should instantiate an maToken
            // and use the Register hook.
            // A new maToken should be created which callbacks this contract in order to be registered.
            let mut addresses_query = address_provider::helpers::query_addresses(
                &deps.querier,
                config.address_provider_address,
                vec![MarsContract::Incentives, MarsContract::ProtocolAdmin],
            )?;

            let protocol_admin_address = addresses_query.pop().unwrap();
            let incentives_address = addresses_query.pop().unwrap();

            let token_symbol = format!("ma{}", symbol);

            let res = Response::new()
                .add_attribute("action", "init_asset")
                .add_attribute("asset", asset_label)
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
                                reference: asset_reference,
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
    asset_type: AssetType,
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
        index,
        asset_type,
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

pub fn execute_init_asset_token_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    reference: Vec<u8>,
) -> Result<Response, ContractError> {
    let mut market = MARKETS.load(deps.storage, reference.as_slice())?;

    if market.ma_token_address == zero_address() {
        let ma_contract_addr = info.sender;

        market.ma_token_address = ma_contract_addr.clone();
        MARKETS.save(deps.storage, reference.as_slice(), &market)?;

        // save ma token contract to reference mapping
        MARKET_REFERENCES_BY_MA_TOKEN.save(deps.storage, &ma_contract_addr, &reference)?;

        let res = Response::new().add_attribute("action", "init_asset_token_callback");
        Ok(res)
    } else {
        // Can do this only once
        Err(MarsError::Unauthorized {}.into())
    }
}

/// Update asset with new params.
pub fn execute_update_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    asset_params: InitOrUpdateAssetParams,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let (asset_label, asset_reference, _asset_type) = asset.get_attributes();
    let market_option = MARKETS.may_load(deps.storage, asset_reference.as_slice())?;
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

            let mut response = Response::new();

            if should_update_interest_rates {
                let protocol_rewards_collector_address = address_provider::helpers::query_address(
                    &deps.querier,
                    config.address_provider_address,
                    MarsContract::ProtocolRewardsCollector,
                )?;
                response = apply_accumulated_interests(
                    &env,
                    protocol_rewards_collector_address,
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

            if should_update_interest_rates {
                response = update_interest_rates(
                    &deps,
                    &env,
                    &mut updated_market,
                    Uint128::zero(),
                    &asset_label,
                    response,
                )?;
            }
            MARKETS.save(deps.storage, asset_reference.as_slice(), &updated_market)?;

            response = response
                .add_attribute("action", "update_asset")
                .add_attribute("asset", &asset_label);

            Ok(response)
        }
    }
}

/// Update uncollateralized loan limit by a given amount in uusd
pub fn execute_update_uncollateralized_loan_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_address: Addr,
    asset: Asset,
    new_limit: Uint128,
) -> Result<Response, ContractError> {
    // Get config
    let config = CONFIG.load(deps.storage)?;

    // Only owner can do this
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let (asset_label, asset_reference, _) = asset.get_attributes();

    // Check that the user has no collateralized debt
    if let Some(user) = USERS.may_load(deps.storage, &user_address)? {
        let previous_uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
            .may_load(deps.storage, (asset_reference.as_slice(), &user_address))?
            .unwrap_or_else(Uint128::zero);

        if previous_uncollateralized_loan_limit == Uint128::zero() {
            let asset_market = MARKETS.load(deps.storage, asset_reference.as_slice())?;

            let is_borrowing_asset = get_bit(user.borrowed_assets, asset_market.index)?;

            if is_borrowing_asset {
                return Err(ContractError::UserHasCollateralizedDebt {});
            }
        };
    }

    UNCOLLATERALIZED_LOAN_LIMITS.save(
        deps.storage,
        (asset_reference.as_slice(), &user_address),
        &new_limit,
    )?;

    DEBTS.update(
        deps.storage,
        (asset_reference.as_slice(), &user_address),
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
        .add_attribute("asset", asset_label)
        .add_attribute("new_allowance", new_limit.to_string());
    Ok(res)
}

/// Execute deposits and mint corresponding ma_tokens
pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender_address: Addr,
    on_behalf_of: Option<String>,
    asset_reference: &[u8],
    asset_label: &str,
    deposit_amount: Uint128,
) -> Result<Response, ContractError> {
    let user_address = if let Some(address) = on_behalf_of {
        deps.api.addr_validate(&address)?
    } else {
        sender_address.clone()
    };

    let mut market = MARKETS.load(deps.storage, asset_reference)?;
    if !market.active {
        return Err(ContractError::MarketNotActive {
            asset: asset_label.to_string(),
        });
    }
    if !market.deposit_enabled {
        return Err(ContractError::DepositNotEnabled {
            asset: asset_label.to_string(),
        });
    }

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(ContractError::InvalidDepositAmount {
            asset: asset_label.to_string(),
        });
    }

    let mut user = USERS
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let mut response = Response::new();
    let has_deposited_asset = get_bit(user.collateral_assets, market.index)?;
    if !has_deposited_asset {
        set_bit(&mut user.collateral_assets, market.index)?;
        USERS.save(deps.storage, &user_address, &user)?;
        response = response.add_event(build_collateral_position_changed_event(
            asset_label,
            true,
            user_address.to_string(),
        ));
    }

    let config = CONFIG.load(deps.storage)?;

    // update indexes and interest rates
    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;
    response = apply_accumulated_interests(
        &env,
        protocol_rewards_collector_address,
        &mut market,
        response,
    )?;
    response = update_interest_rates(
        &deps,
        &env,
        &mut market,
        Uint128::zero(),
        asset_label,
        response,
    )?;
    MARKETS.save(deps.storage, asset_reference, &market)?;

    if market.liquidity_index.is_zero() {
        return Err(ContractError::InvalidLiquidityIndex {});
    }
    let mint_amount =
        get_scaled_liquidity_amount(deposit_amount, &market, env.block.time.seconds())?;

    response = response
        .add_attribute("action", "deposit")
        .add_attribute("asset", asset_label)
        .add_attribute("sender", sender_address)
        .add_attribute("user", user_address.as_str())
        .add_attribute("amount", deposit_amount)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market.ma_token_address.into(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: user_address.into(),
                amount: mint_amount,
            })?,
            funds: vec![],
        }));

    Ok(response)
}

/// Burns sent maAsset in exchange of underlying asset
pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    amount: Option<Uint128>,
    recipient_address: Option<String>,
) -> Result<Response, ContractError> {
    let withdrawer_addr = info.sender;

    let (asset_label, asset_reference, asset_type) = asset.get_attributes();
    let mut market = MARKETS.load(deps.storage, asset_reference.as_slice())?;

    if !market.active {
        return Err(ContractError::MarketNotActive { asset: asset_label });
    }

    let asset_ma_addr = market.ma_token_address.clone();
    let withdrawer_balance_scaled =
        cw20_get_balance(&deps.querier, asset_ma_addr, withdrawer_addr.clone())?;

    if withdrawer_balance_scaled.is_zero() {
        return Err(ContractError::UserNoBalance { asset: asset_label });
    }

    let (withdraw_amount, withdraw_amount_scaled) = match amount {
        Some(amount) => {
            // Check user has sufficient balance to send back
            let amount_scaled =
                get_scaled_liquidity_amount(amount, &market, env.block.time.seconds())?;
            if amount_scaled.is_zero() || amount_scaled > withdrawer_balance_scaled {
                return Err(ContractError::InvalidWithdrawAmount { asset: asset_label });
            };
            (amount, amount_scaled)
        }
        None => {
            // If no amount is specified, the full balance is withdrawn
            let withdrawer_balance = get_underlying_liquidity_amount(
                withdrawer_balance_scaled,
                &market,
                env.block.time.seconds(),
            )?;
            (withdrawer_balance, withdrawer_balance_scaled)
        }
    };

    let config = CONFIG.load(deps.storage)?;

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = addresses_query.pop().unwrap();
    let oracle_address = addresses_query.pop().unwrap();

    let mut withdrawer = match USERS.may_load(deps.storage, &withdrawer_addr)? {
        Some(user) => user,
        None => {
            // No address should withdraw without an existing user position already in
            // storage (If this happens the protocol did something wrong). The exception is
            // the protocol_rewards_collector which gets minted token without depositing
            // nor receiving a transfer from another user.
            if withdrawer_addr != protocol_rewards_collector_address {
                return Err(ContractError::ExistingUserPositionRequired {});
            }
            User::default()
        }
    };
    let asset_as_collateral = get_bit(withdrawer.collateral_assets, market.index)?;
    let user_is_borrowing = !withdrawer.borrowed_assets.is_zero();

    // if asset is used as collateral and user is borrowing we need to validate health factor after withdraw,
    // otherwise no reasons to block the withdraw
    if asset_as_collateral && user_is_borrowing {
        let global_state = GLOBAL_STATE.load(deps.storage)?;

        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &withdrawer_addr,
            oracle_address,
            &withdrawer,
            global_state.market_count,
        )?;

        let withdraw_asset_price =
            user_position.get_asset_price(asset_reference.as_slice(), &asset_label)?;

        let withdraw_amount_in_uusd = withdraw_amount * withdraw_asset_price;

        let weighted_liquidation_threshold_in_uusd_after_withdraw = user_position
            .weighted_liquidation_threshold_in_uusd
            .checked_sub(withdraw_amount_in_uusd * market.liquidation_threshold)?;
        let health_factor_after_withdraw = Decimal::from_ratio(
            weighted_liquidation_threshold_in_uusd_after_withdraw,
            user_position.total_collateralized_debt_in_uusd,
        );
        if health_factor_after_withdraw < Decimal::one() {
            return Err(ContractError::InvalidHealthFactorAfterWithdraw {});
        }
    }

    let mut response = Response::new();

    // if amount to withdraw equals the user's balance then unset collateral bit
    if asset_as_collateral && withdraw_amount_scaled == withdrawer_balance_scaled {
        unset_bit(&mut withdrawer.collateral_assets, market.index)?;
        USERS.save(deps.storage, &withdrawer_addr, &withdrawer)?;
        response = response.add_event(build_collateral_position_changed_event(
            asset_label.as_str(),
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
    response = update_interest_rates(
        &deps,
        &env,
        &mut market,
        withdraw_amount,
        &asset_label,
        response,
    )?;
    MARKETS.save(deps.storage, asset_reference.as_slice(), &market)?;

    // burn maToken
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: market.ma_token_address.to_string(),
        msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
            user: withdrawer_addr.to_string(),
            amount: withdraw_amount_scaled,
        })?,
        funds: vec![],
    }));

    // send underlying asset to user or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        withdrawer_addr.clone()
    };
    response = response.add_message(build_send_asset_with_tax_deduction_msg(
        deps.as_ref(),
        recipient_address.clone(),
        asset_label.clone(),
        asset_type,
        withdraw_amount,
    )?);

    response = response
        .add_attribute("action", "withdraw")
        .add_attribute("asset", asset_label.as_str())
        .add_attribute("user", withdrawer_addr.as_str())
        .add_attribute("recipient", recipient_address.as_str())
        .add_attribute("burn_amount", withdraw_amount_scaled)
        .add_attribute("withdraw_amount", withdraw_amount);
    Ok(response)
}

/// Add debt for the borrower and send the borrowed funds
pub fn execute_borrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    borrow_amount: Uint128,
    recipient_address: Option<String>,
) -> Result<Response, ContractError> {
    let borrower_address = info.sender;
    let (asset_label, asset_reference, asset_type) = asset.get_attributes();

    // Cannot borrow zero amount
    if borrow_amount.is_zero() {
        return Err(ContractError::InvalidBorrowAmount { asset: asset_label });
    }

    // Load market and user state
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let mut borrow_market = MARKETS.load(deps.storage, asset_reference.as_slice())?;

    if !borrow_market.active {
        return Err(ContractError::MarketNotActive { asset: asset_label });
    }
    if !borrow_market.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled { asset: asset_label });
    }

    let uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(
            deps.storage,
            (asset_reference.as_slice(), &borrower_address),
        )?
        .unwrap_or_else(Uint128::zero);
    let mut user: User = match USERS.may_load(deps.storage, &borrower_address)? {
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

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = addresses_query.pop().unwrap();
    let oracle_address = addresses_query.pop().unwrap();

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if uncollateralized_loan_limit.is_zero() {
        // Collateralized loan: check max ltv is not exceeded
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &borrower_address,
            oracle_address.clone(),
            &user,
            global_state.market_count,
        )?;

        let borrow_asset_price = if is_borrowing_asset {
            // if user was already borrowing, get price from user position
            user_position.get_asset_price(asset_reference.as_slice(), &asset_label)?
        } else {
            mars_core::oracle::helpers::query_price(
                deps.querier,
                oracle_address,
                &asset_label,
                asset_reference.clone(),
                asset_type,
            )?
        };

        let borrow_amount_in_uusd = borrow_amount * borrow_asset_price;

        let total_debt_in_uusd_after_borrow = user_position
            .total_debt_in_uusd
            .checked_add(borrow_amount_in_uusd)?;
        if total_debt_in_uusd_after_borrow > user_position.max_debt_in_uusd {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    } else {
        // Uncollateralized loan: check borrow amount plus debt does not exceed uncollateralized loan limit
        uncollateralized_debt = true;

        let borrower_debt = DEBTS
            .may_load(
                deps.storage,
                (asset_reference.as_slice(), &borrower_address),
            )?
            .unwrap_or(Debt {
                amount_scaled: Uint128::zero(),
                uncollateralized: uncollateralized_debt,
            });

        let asset_market = MARKETS.load(deps.storage, asset_reference.as_slice())?;
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
        USERS.save(deps.storage, &borrower_address, &user)?;
        response = response.add_event(build_debt_position_changed_event(
            asset_label.as_str(),
            true,
            borrower_address.to_string(),
        ));
    }

    // Set new debt
    let mut debt = DEBTS
        .may_load(
            deps.storage,
            (asset_reference.as_slice(), &borrower_address),
        )?
        .unwrap_or(Debt {
            amount_scaled: Uint128::zero(),
            uncollateralized: uncollateralized_debt,
        });
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;
    debt.amount_scaled = debt.amount_scaled.checked_add(borrow_amount_scaled)?;
    DEBTS.save(
        deps.storage,
        (asset_reference.as_slice(), &borrower_address),
        &debt,
    )?;

    borrow_market.debt_total_scaled += borrow_amount_scaled;

    response = update_interest_rates(
        &deps,
        &env,
        &mut borrow_market,
        borrow_amount,
        &asset_label,
        response,
    )?;
    MARKETS.save(deps.storage, asset_reference.as_slice(), &borrow_market)?;

    // Send borrow amount to borrower or another recipient
    let recipient_address = if let Some(address) = recipient_address {
        deps.api.addr_validate(&address)?
    } else {
        borrower_address.clone()
    };
    response = response.add_message(build_send_asset_with_tax_deduction_msg(
        deps.as_ref(),
        recipient_address.clone(),
        asset_label.clone(),
        asset_type,
        borrow_amount,
    )?);

    response = response
        .add_attribute("action", "borrow")
        .add_attribute("asset", asset_label.as_str())
        .add_attribute("user", borrower_address.as_str())
        .add_attribute("recipient", recipient_address.as_str())
        .add_attribute("amount", borrow_amount);
    Ok(response)
}

/// Handle the repay of native tokens. Refund extra funds if they exist
pub fn execute_repay(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender_address: Addr,
    on_behalf_of: Option<String>,
    asset_reference: &[u8],
    asset_label: String,
    repay_amount: Uint128,
    asset_type: AssetType,
) -> Result<Response, ContractError> {
    let user_address = if let Some(address) = on_behalf_of {
        let on_behalf_of_addr = deps.api.addr_validate(&address)?;
        // Uncollateralized loans should not have 'on behalf of' because it creates accounting complexity for them
        match UNCOLLATERALIZED_LOAN_LIMITS
            .may_load(deps.storage, (asset_reference, &on_behalf_of_addr))?
        {
            Some(limit) if !limit.is_zero() => {
                return Err(ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {})
            }
            _ => on_behalf_of_addr,
        }
    } else {
        sender_address.clone()
    };

    let mut market = MARKETS.load(deps.storage, asset_reference)?;

    if !market.active {
        return Err(ContractError::MarketNotActive { asset: asset_label });
    }

    // Cannot repay zero amount
    if repay_amount.is_zero() {
        return Err(ContractError::InvalidRepayAmount { asset: asset_label });
    }

    // Check new debt
    let mut debt = DEBTS.load(deps.storage, (asset_reference, &user_address))?;

    if debt.amount_scaled.is_zero() {
        return Err(ContractError::CannotRepayZeroDebt {});
    }

    let config = CONFIG.load(deps.storage)?;

    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;

    let mut response = Response::new();

    response = apply_accumulated_interests(
        &env,
        protocol_rewards_collector_address,
        &mut market,
        response,
    )?;

    let mut repay_amount_scaled =
        get_scaled_debt_amount(repay_amount, &market, env.block.time.seconds())?;

    // If repay amount exceeds debt, refund any excess amounts
    let mut refund_amount = Uint128::zero();
    if repay_amount_scaled > debt.amount_scaled {
        refund_amount = get_underlying_debt_amount(
            repay_amount_scaled - debt.amount_scaled,
            &market,
            env.block.time.seconds(),
        )?;
        let refund_msg = build_send_asset_with_tax_deduction_msg(
            deps.as_ref(),
            user_address.clone(),
            asset_label.clone(),
            asset_type,
            refund_amount,
        )?;
        response = response.add_message(refund_msg);
        repay_amount_scaled = debt.amount_scaled;
    }

    debt.amount_scaled = debt.amount_scaled.checked_sub(repay_amount_scaled)?;
    DEBTS.save(deps.storage, (asset_reference, &user_address), &debt)?;

    if repay_amount_scaled > market.debt_total_scaled {
        return Err(ContractError::CannotRepayMoreThanDebt {});
    }
    market.debt_total_scaled = market.debt_total_scaled.checked_sub(repay_amount_scaled)?;

    response = update_interest_rates(
        &deps,
        &env,
        &mut market,
        Uint128::zero(),
        &asset_label,
        response,
    )?;
    MARKETS.save(deps.storage, asset_reference, &market)?;

    if debt.amount_scaled.is_zero() {
        // Remove asset from borrowed assets
        let mut user = USERS.load(deps.storage, &user_address)?;
        unset_bit(&mut user.borrowed_assets, market.index)?;
        USERS.save(deps.storage, &user_address, &user)?;
        response = response.add_event(build_debt_position_changed_event(
            &asset_label,
            false,
            user_address.to_string(),
        ));
    }

    response = response
        .add_attribute("action", "repay")
        .add_attribute("asset", asset_label)
        .add_attribute("sender", sender_address)
        .add_attribute("user", user_address)
        .add_attribute("amount", repay_amount.checked_sub(refund_amount)?);
    Ok(response)
}

/// Execute loan liquidations on under-collateralized loans
pub fn execute_liquidate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    liquidator_address: Addr,
    collateral_asset: Asset,
    debt_asset: Asset,
    user_address: Addr,
    sent_debt_asset_amount: Uint128,
    receive_ma_token: bool,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let (debt_asset_label, debt_asset_reference, debt_asset_type) = debt_asset.get_attributes();

    // 1. Validate liquidation
    // If user (contract) has a positive uncollateralized limit then the user
    // cannot be liquidated
    if let Some(limit) = UNCOLLATERALIZED_LOAN_LIMITS.may_load(
        deps.storage,
        (debt_asset_reference.as_slice(), &user_address),
    )? {
        if !limit.is_zero() {
            return Err(ContractError::CannotLiquidateWhenPositiveUncollateralizedLoanLimit {});
        }
    };

    // liquidator must send positive amount of funds in the debt asset
    if sent_debt_asset_amount.is_zero() {
        return Err(ContractError::InvalidLiquidateAmount {
            asset: debt_asset_label,
        });
    }

    let (collateral_asset_label, collateral_asset_reference, collateral_asset_type) =
        collateral_asset.get_attributes();

    let collateral_market = MARKETS.load(deps.storage, collateral_asset_reference.as_slice())?;

    if !collateral_market.active {
        return Err(ContractError::MarketNotActive {
            asset: collateral_asset_label,
        });
    }

    let mut user = USERS.load(deps.storage, &user_address)?;
    let using_collateral_asset_as_collateral =
        get_bit(user.collateral_assets, collateral_market.index)?;
    if !using_collateral_asset_as_collateral {
        return Err(ContractError::CannotLiquidateWhenCollateralUnset {
            asset: collateral_asset_label,
        });
    }

    // check if user has available collateral in specified collateral asset to be liquidated
    let user_collateral_balance_scaled = cw20_get_balance(
        &deps.querier,
        collateral_market.ma_token_address.clone(),
        user_address.clone(),
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
    let user_debt = DEBTS.load(
        deps.storage,
        (debt_asset_reference.as_slice(), &user_address),
    )?;
    if user_debt.amount_scaled.is_zero() {
        return Err(ContractError::CannotLiquidateWhenNoDebtBalance {});
    }

    // 2. Compute health factor
    let config = CONFIG.load(deps.storage)?;

    let mut addresses_query = address_provider::helpers::query_addresses(
        &deps.querier,
        config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = addresses_query.pop().unwrap();
    let oracle_address = addresses_query.pop().unwrap();

    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let user_position = get_user_position(
        deps.as_ref(),
        block_time,
        &user_address,
        oracle_address,
        &user,
        global_state.market_count,
    )?;

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

    let collateral_and_debt_are_the_same_asset = debt_asset_type == collateral_asset_type
        && debt_asset_reference == collateral_asset_reference;

    let debt_market = if !collateral_and_debt_are_the_same_asset {
        MARKETS.load(deps.storage, debt_asset_reference.as_slice())?
    } else {
        collateral_market.clone()
    };

    if !debt_market.active {
        return Err(ContractError::MarketNotActive {
            asset: debt_asset_label,
        });
    }

    // 3. Compute debt to repay and collateral to liquidate
    let collateral_price = user_position.get_asset_price(
        collateral_asset_reference.as_slice(),
        &collateral_asset_label,
    )?;
    let debt_price =
        user_position.get_asset_price(debt_asset_reference.as_slice(), &debt_asset_label)?;

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
            &user_address,
            &liquidator_address,
            &collateral_asset_label,
            &collateral_market,
            collateral_amount_to_liquidate,
            response,
        )?;
    } else {
        response = process_underlying_asset_transfer_to_liquidator(
            deps.branch(),
            &env,
            &user_address,
            &liquidator_address,
            collateral_asset_label.clone(),
            collateral_asset_type,
            &collateral_market,
            collateral_amount_to_liquidate,
            response,
        )?;
    }

    // if max collateral to liquidate equals the user's balance then unset collateral bit
    if collateral_amount_to_liquidate == user_collateral_balance {
        unset_bit(&mut user.collateral_assets, collateral_market.index)?;
        USERS.save(deps.storage, &user_address, &user)?;
        response = response.add_event(build_collateral_position_changed_event(
            collateral_asset_label.as_str(),
            false,
            user_address.to_string(),
        ));
    }

    // 5. Compute and update new debt
    let debt_amount_to_repay_scaled =
        get_scaled_debt_amount(debt_amount_to_repay, &debt_market, env.block.time.seconds())?;

    // update user and market debt
    let mut debt = DEBTS.load(
        deps.storage,
        (debt_asset_reference.as_slice(), &user_address),
    )?;
    // NOTE: Should be > 0 as amount to repay is capped by the close factor
    debt.amount_scaled = debt
        .amount_scaled
        .checked_sub(debt_amount_to_repay_scaled)?;
    DEBTS.save(
        deps.storage,
        (debt_asset_reference.as_slice(), &user_address),
        &debt,
    )?;

    let debt_market_debt_total_scaled_after = debt_market
        .debt_total_scaled
        .checked_sub(debt_amount_to_repay_scaled)?;

    // 6. Update markets depending on whether the collateral and debt markets are the same
    // and whether the liquidator receives ma_tokens (no change in liquidity) or underlying asset
    // (changes liquidity)
    if collateral_and_debt_are_the_same_asset {
        // NOTE: for the sake of clarity copy attributes from collateral market and
        // give generic naming. Debt market could have been used as well
        let mut asset_market_after = collateral_market;
        let asset_reference = collateral_asset_reference;
        let asset_label = &collateral_asset_label;

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
            asset_label,
            response,
        )?;

        MARKETS.save(
            deps.storage,
            asset_reference.as_slice(),
            &asset_market_after,
        )?;
    } else {
        if !receive_ma_token {
            let mut collateral_market_after = collateral_market;

            response = apply_accumulated_interests(
                &env,
                protocol_rewards_collector_address.clone(),
                &mut collateral_market_after,
                response,
            )?;

            response = update_interest_rates(
                &deps,
                &env,
                &mut collateral_market_after,
                collateral_amount_to_liquidate,
                &collateral_asset_label,
                response,
            )?;

            MARKETS.save(
                deps.storage,
                collateral_asset_reference.as_slice(),
                &collateral_market_after,
            )?;
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
            &debt_asset_label,
            response,
        )?;

        MARKETS.save(
            deps.storage,
            debt_asset_reference.as_slice(),
            &debt_market_after,
        )?;
    }

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    if refund_amount > Uint128::zero() {
        response = response.add_message(build_send_asset_with_tax_deduction_msg(
            deps.as_ref(),
            liquidator_address.clone(),
            debt_asset_label.clone(),
            debt_asset_type,
            refund_amount,
        )?);
    }

    response = response
        .add_attribute("action", "liquidate")
        .add_attribute("collateral_asset", collateral_asset_label.as_str())
        .add_attribute("debt_asset", debt_asset_label.as_str())
        .add_attribute("user", user_address.as_str())
        .add_attribute("liquidator", liquidator_address.as_str())
        .add_attribute(
            "collateral_amount_liquidated",
            collateral_amount_to_liquidate.to_string(),
        )
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
    collateral_asset_label: &str,
    collateral_market: &Market,
    collateral_amount_to_liquidate: Uint128,
    mut response: Response,
) -> StdResult<Response> {
    let mut liquidator = USERS
        .may_load(deps.storage, liquidator_addr)?
        .unwrap_or_default();

    // Set liquidator's deposited bit to true if not already true
    // NOTE: previous checks should ensure amount to be sent is not zero
    let liquidator_is_using_as_collateral =
        get_bit(liquidator.collateral_assets, collateral_market.index)?;
    if !liquidator_is_using_as_collateral {
        set_bit(&mut liquidator.collateral_assets, collateral_market.index)?;
        USERS.save(deps.storage, liquidator_addr, &liquidator)?;
        response = response.add_event(build_collateral_position_changed_event(
            collateral_asset_label,
            true,
            liquidator_addr.to_string(),
        ));
    }

    let collateral_amount_to_liquidate_scaled = get_scaled_liquidity_amount(
        collateral_amount_to_liquidate,
        collateral_market,
        block_time,
    )?;

    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collateral_market.ma_token_address.to_string(),
        msg: to_binary(
            &mars_core::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                sender: user_addr.to_string(),
                recipient: liquidator_addr.to_string(),
                amount: collateral_amount_to_liquidate_scaled,
            },
        )?,
        funds: vec![],
    }));

    Ok(response)
}

/// Burn ma_tokens from user and send underlying asset to liquidator
/// Returns response with added messages and events
fn process_underlying_asset_transfer_to_liquidator(
    deps: DepsMut,
    env: &Env,
    user_addr: &Addr,
    liquidator_addr: &Addr,
    collateral_asset_label: String,
    collateral_asset_type: AssetType,
    collateral_market: &Market,
    collateral_amount_to_liquidate: Uint128,
    mut response: Response,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();

    // Ensure contract has enough collateral to send back underlying asset
    let contract_collateral_balance = get_asset_balance(
        deps.as_ref(),
        env.contract.address.clone(),
        collateral_asset_label.clone(),
        collateral_asset_type,
    )?;

    if contract_collateral_balance < collateral_amount_to_liquidate {
        return Err(ContractError::CannotLiquidateWhenNotEnoughCollateral {});
    }

    let collateral_amount_to_liquidate_scaled = get_scaled_liquidity_amount(
        collateral_amount_to_liquidate,
        collateral_market,
        block_time,
    )?;

    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: collateral_market.ma_token_address.to_string(),
        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
            user: user_addr.to_string(),

            amount: collateral_amount_to_liquidate_scaled,
        })?,
        funds: vec![],
    }));

    response = response.add_message(build_send_asset_with_tax_deduction_msg(
        deps.as_ref(),
        liquidator_addr.clone(),
        collateral_asset_label,
        collateral_asset_type,
        collateral_amount_to_liquidate,
    )?);

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

    // Collateral: debt to repay in uusd times the liquidation
    // bonus
    let debt_amount_to_repay_in_uusd = debt_amount_to_repay * debt_price;
    let collateral_amount_to_liquidate_in_uusd =
        debt_amount_to_repay_in_uusd * (Decimal::one() + liquidation_bonus);
    let mut collateral_amount_to_liquidate = Decimal::divide_uint128_by_decimal(
        collateral_amount_to_liquidate_in_uusd,
        collateral_price,
    )?;

    // If collateral amount to liquidate is higher than user_collateral_balance,
    // liquidate the full balance and adjust the debt amount to repay accordingly
    if collateral_amount_to_liquidate > user_collateral_balance {
        collateral_amount_to_liquidate = user_collateral_balance;
        debt_amount_to_repay = Decimal::divide_uint128_by_decimal(
            Decimal::divide_uint128_by_decimal(
                collateral_amount_to_liquidate * collateral_price,
                debt_price,
            )?,
            Decimal::one() + liquidation_bonus,
        )?
    }

    let refund_amount = sent_debt_asset_amount - debt_amount_to_repay;

    Ok((
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        refund_amount,
    ))
}

/// Update (enable / disable) collateral asset for specific user
pub fn execute_update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    enable: bool,
) -> Result<Response, ContractError> {
    let user_address = info.sender;
    let mut user = USERS
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let mut events = vec![];

    let (collateral_asset_label, collateral_asset_reference, _) = asset.get_attributes();
    let collateral_market = MARKETS.load(deps.storage, collateral_asset_reference.as_slice())?;
    let has_collateral_asset = get_bit(user.collateral_assets, collateral_market.index)?;
    if !has_collateral_asset && enable {
        let collateral_ma_address = collateral_market.ma_token_address;
        let user_collateral_balance =
            cw20_get_balance(&deps.querier, collateral_ma_address, user_address.clone())?;
        if user_collateral_balance > Uint128::zero() {
            // enable collateral asset
            set_bit(&mut user.collateral_assets, collateral_market.index)?;
            USERS.save(deps.storage, &user_address, &user)?;
            events.push(build_collateral_position_changed_event(
                collateral_asset_label.as_str(),
                true,
                user_address.to_string(),
            ));
        } else {
            return Err(ContractError::UserNoCollateralBalance {
                user_address: user_address.to_string(),
                asset: collateral_asset_label,
            });
        }
    } else if has_collateral_asset && !enable {
        // disable collateral asset
        unset_bit(&mut user.collateral_assets, collateral_market.index)?;

        // check health factor after disabling collateral
        let global_state = GLOBAL_STATE.load(deps.storage)?;
        let config = CONFIG.load(deps.storage)?;
        let oracle_address = address_provider::helpers::query_address(
            &deps.querier,
            config.address_provider_address,
            MarsContract::Oracle,
        )?;
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &user_address,
            oracle_address,
            &user,
            global_state.market_count,
        )?;
        // if health factor is less than one after disabling collateral we can't process further
        if let UserHealthStatus::Borrowing(health_factor) = user_position.health_status {
            if health_factor < Decimal::one() {
                return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
            }
        }

        USERS.save(deps.storage, &user_address, &user)?;
        events.push(build_collateral_position_changed_event(
            collateral_asset_label.as_str(),
            false,
            user_address.to_string(),
        ));
    }

    let res = Response::new()
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user_address.as_str())
        .add_attribute("asset", collateral_asset_label)
        .add_attribute("has_collateral", has_collateral_asset.to_string())
        .add_attribute("enable", enable.to_string())
        .add_events(events);
    Ok(res)
}

/// Update uncollateralized loan limit by a given amount in uusd
pub fn execute_finalize_liquidity_token_transfer(
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
    let market_reference = MARKET_REFERENCES_BY_MA_TOKEN.load(deps.storage, &info.sender)?;
    let market = MARKETS.load(deps.storage, market_reference.as_slice())?;

    // Check user health factor is above 1
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let mut from_user = USERS.load(deps.storage, &from_address)?;
    let config = CONFIG.load(deps.storage)?;
    let oracle_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position = get_user_position(
        deps.as_ref(),
        env.block.time.seconds(),
        &from_address,
        oracle_address,
        &from_user,
        global_state.market_count,
    )?;
    if let UserHealthStatus::Borrowing(health_factor) = user_position.health_status {
        if health_factor < Decimal::one() {
            return Err(ContractError::CannotTransferTokenWhenInvalidHealthFactor {});
        }
    }

    let asset_label = String::from_utf8(market_reference).expect("Found invalid UTF-8");
    let mut events = vec![];

    // Update users's positions
    if from_address != to_address {
        if from_previous_balance.checked_sub(amount)?.is_zero() {
            unset_bit(&mut from_user.collateral_assets, market.index)?;
            USERS.save(deps.storage, &from_address, &from_user)?;
            events.push(build_collateral_position_changed_event(
                asset_label.as_str(),
                false,
                from_address.to_string(),
            ))
        }

        if to_previous_balance.is_zero() && !amount.is_zero() {
            let mut to_user = USERS
                .may_load(deps.storage, &to_address)?
                .unwrap_or_default();
            set_bit(&mut to_user.collateral_assets, market.index)?;
            USERS.save(deps.storage, &to_address, &to_user)?;
            events.push(build_collateral_position_changed_event(
                asset_label.as_str(),
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

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),

        QueryMsg::Market { asset } => to_binary(&query_market(deps, asset)?),

        QueryMsg::MarketsList {} => to_binary(&query_markets_list(deps)?),

        QueryMsg::UserDebt { user_address } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_user_debt(deps, env, address)?)
        }

        QueryMsg::UserAssetDebt {
            user_address,
            asset,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_user_asset_debt(deps, env, address, asset)?)
        }

        QueryMsg::UserCollateral { user_address } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_user_collateral(deps, address)?)
        }

        QueryMsg::UncollateralizedLoanLimit {
            user_address,
            asset,
        } => {
            let user_address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_uncollateralized_loan_limit(
                deps,
                user_address,
                asset,
            )?)
        }

        QueryMsg::ScaledLiquidityAmount { asset, amount } => {
            to_binary(&query_scaled_liquidity_amount(deps, env, asset, amount)?)
        }

        QueryMsg::ScaledDebtAmount { asset, amount } => {
            to_binary(&query_scaled_debt_amount(deps, env, asset, amount)?)
        }

        QueryMsg::UnderlyingLiquidityAmount {
            ma_token_address,
            amount_scaled,
        } => to_binary(&query_underlying_liquidity_amount(
            deps,
            env,
            ma_token_address,
            amount_scaled,
        )?),

        QueryMsg::UnderlyingDebtAmount {
            asset,
            amount_scaled,
        } => to_binary(&query_underlying_debt_amount(
            deps,
            env,
            asset,
            amount_scaled,
        )?),

        QueryMsg::UserPosition { user_address } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_user_position(deps, env, address)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let money_market = GLOBAL_STATE.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner,
        address_provider_address: config.address_provider_address,
        ma_token_code_id: config.ma_token_code_id,
        market_count: money_market.market_count,
        close_factor: config.close_factor,
    })
}

pub fn query_market(deps: Deps, asset: Asset) -> StdResult<Market> {
    let (label, reference, _) = asset.get_attributes();
    let market = match MARKETS.load(deps.storage, reference.as_slice()) {
        Ok(market) => market,
        Err(_) => {
            return Err(StdError::generic_err(format!(
                "failed to load market for: {}",
                label
            )))
        }
    };

    Ok(market)
}

pub fn query_markets_list(deps: Deps) -> StdResult<MarketsListResponse> {
    let markets_list: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (asset_reference, market) = item?;
            let (denom, asset_label) =
                get_asset_identifiers(deps, asset_reference.clone(), market.asset_type)?;

            Ok(MarketInfo {
                denom,
                asset_label,
                asset_reference,
                asset_type: market.asset_type,
                ma_token_address: market.ma_token_address,
            })
        })
        .collect();

    Ok(MarketsListResponse {
        markets_list: markets_list?,
    })
}

pub fn query_user_debt(deps: Deps, env: Env, user_address: Addr) -> StdResult<UserDebtResponse> {
    let user = USERS
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let debts: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (asset_reference, market) = item?;
            let (denom, asset_label) =
                get_asset_identifiers(deps, asset_reference.clone(), market.asset_type)?;

            let is_borrowing_asset = get_bit(user.borrowed_assets, market.index)?;
            let (amount_scaled, amount) = if is_borrowing_asset {
                let debt = DEBTS.load(deps.storage, (asset_reference.as_slice(), &user_address))?;
                let amount_scaled = debt.amount_scaled;
                let amount =
                    get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;
                (amount_scaled, amount)
            } else {
                (Uint128::zero(), Uint128::zero())
            };

            Ok(UserAssetDebtResponse {
                denom,
                asset_label,
                asset_reference,
                asset_type: market.asset_type,
                amount_scaled,
                amount,
            })
        })
        .collect();

    Ok(UserDebtResponse { debts: debts? })
}

pub fn query_user_asset_debt(
    deps: Deps,
    env: Env,
    user_address: Addr,
    asset: Asset,
) -> StdResult<UserAssetDebtResponse> {
    let (asset_label, asset_reference, asset_type) = asset.get_attributes();

    let market = MARKETS.load(deps.storage, &asset_reference)?;

    let denom = get_asset_denom(deps, &asset_label, asset_type)?;

    let (amount_scaled, amount) =
        match DEBTS.may_load(deps.storage, (asset_reference.as_slice(), &user_address))? {
            Some(debt) => {
                let amount_scaled = debt.amount_scaled;
                let amount =
                    get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())?;
                (amount_scaled, amount)
            }

            None => (Uint128::zero(), Uint128::zero()),
        };

    Ok(UserAssetDebtResponse {
        denom,
        asset_label,
        asset_reference,
        asset_type: market.asset_type,
        amount_scaled,
        amount,
    })
}

pub fn query_user_collateral(deps: Deps, address: Addr) -> StdResult<UserCollateralResponse> {
    let user = USERS.may_load(deps.storage, &address)?.unwrap_or_default();

    let collateral: StdResult<Vec<_>> = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (asset_reference, market) = item?;
            let (denom, asset_label) =
                get_asset_identifiers(deps, asset_reference.clone(), market.asset_type)?;

            Ok(UserAssetCollateralResponse {
                denom,
                asset_label,
                asset_reference,
                asset_type: market.asset_type,
                enabled: get_bit(user.collateral_assets, market.index)?,
            })
        })
        .collect();

    Ok(UserCollateralResponse {
        collateral: collateral?,
    })
}

pub fn query_uncollateralized_loan_limit(
    deps: Deps,
    user_address: Addr,
    asset: Asset,
) -> StdResult<Uint128> {
    let (asset_label, asset_reference, _) = asset.get_attributes();
    let uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .load(deps.storage, (asset_reference.as_slice(), &user_address));

    match uncollateralized_loan_limit {
        Ok(limit) => Ok(limit),
        Err(_) => Err(StdError::not_found(format!(
            "No uncollateralized loan approved for user_address: {} on asset: {}",
            user_address, asset_label
        ))),
    }
}

pub fn query_scaled_liquidity_amount(
    deps: Deps,
    env: Env,
    asset: Asset,
    amount: Uint128,
) -> StdResult<Uint128> {
    let asset_reference = asset.get_reference();
    let market = MARKETS.load(deps.storage, asset_reference.as_slice())?;
    get_scaled_liquidity_amount(amount, &market, env.block.time.seconds())
}

pub fn query_scaled_debt_amount(
    deps: Deps,
    env: Env,
    asset: Asset,
    amount: Uint128,
) -> StdResult<Uint128> {
    let asset_reference = asset.get_reference();
    let market = MARKETS.load(deps.storage, asset_reference.as_slice())?;
    get_scaled_debt_amount(amount, &market, env.block.time.seconds())
}

pub fn query_underlying_liquidity_amount(
    deps: Deps,
    env: Env,
    ma_token_address: String,
    amount_scaled: Uint128,
) -> StdResult<Uint128> {
    let ma_token_address = deps.api.addr_validate(&ma_token_address)?;
    let market_reference = MARKET_REFERENCES_BY_MA_TOKEN.load(deps.storage, &ma_token_address)?;
    let market = MARKETS.load(deps.storage, market_reference.as_slice())?;
    get_underlying_liquidity_amount(amount_scaled, &market, env.block.time.seconds())
}

pub fn query_underlying_debt_amount(
    deps: Deps,
    env: Env,
    asset: Asset,
    amount_scaled: Uint128,
) -> StdResult<Uint128> {
    let asset_reference = asset.get_reference();
    let market = MARKETS.load(deps.storage, asset_reference.as_slice())?;
    get_underlying_debt_amount(amount_scaled, &market, env.block.time.seconds())
}

pub fn query_user_position(
    deps: Deps,
    env: Env,
    address: Addr,
) -> Result<UserPositionResponse, MarsError> {
    let config = CONFIG.load(deps.storage)?;
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let user = USERS.load(deps.storage, &address)?;
    let oracle_address = address_provider::helpers::query_address(
        &deps.querier,
        config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position = get_user_position(
        deps,
        env.block.time.seconds(),
        &address,
        oracle_address,
        &user,
        global_state.market_count,
    )?;

    Ok(UserPositionResponse {
        total_collateral_in_uusd: user_position.total_collateral_in_uusd,
        total_debt_in_uusd: user_position.total_debt_in_uusd,
        total_collateralized_debt_in_uusd: user_position.total_collateralized_debt_in_uusd,
        max_debt_in_uusd: user_position.max_debt_in_uusd,
        weighted_liquidation_threshold_in_uusd: user_position
            .weighted_liquidation_threshold_in_uusd,
        health_status: user_position.health_status,
    })
}

// EVENTS

fn build_collateral_position_changed_event(label: &str, enabled: bool, user_addr: String) -> Event {
    Event::new("collateral_position_changed")
        .add_attribute("asset", label)
        .add_attribute("using_as_collateral", enabled.to_string())
        .add_attribute("user", user_addr)
}

fn build_debt_position_changed_event(label: &str, enabled: bool, user_addr: String) -> Event {
    Event::new("debt_position_changed")
        .add_attribute("asset", label)
        .add_attribute("borrowing", enabled.to_string())
        .add_attribute("user", user_addr)
}

// HELPERS

// native coins
fn get_denom_amount_from_coins(coins: &[Coin], denom: &str) -> Result<Uint128, ContractError> {
    if coins.len() == 1 && coins[0].denom == denom {
        Ok(coins[0].amount)
    } else {
        Err(InvalidNativeCoinsSent {
            denom: denom.to_string(),
        })
    }
}

fn get_asset_identifiers(
    deps: Deps,
    asset_reference: Vec<u8>,
    asset_type: AssetType,
) -> StdResult<(String, String)> {
    let asset_label = String::from_utf8(asset_reference)?;
    let denom = get_asset_denom(deps, &asset_label, asset_type)?;
    Ok((denom, asset_label))
}

fn get_asset_denom(deps: Deps, asset_label: &str, asset_type: AssetType) -> StdResult<String> {
    match asset_type {
        AssetType::Native => Ok(asset_label.to_string()),
        AssetType::Cw20 => {
            let cw20_contract_address = deps.api.addr_validate(asset_label)?;
            match cw20_get_symbol(&deps.querier, cw20_contract_address.clone()) {
                Ok(symbol) => Ok(symbol),
                Err(_) => {
                    return Err(StdError::generic_err(format!(
                        "failed to get symbol from cw20 contract address: {}",
                        cw20_contract_address
                    )));
                }
            }
        }
    }
}

pub fn market_get_from_index(deps: &Deps, index: u32) -> StdResult<(Vec<u8>, Market)> {
    let asset_reference_vec =
        match MARKET_REFERENCES_BY_INDEX.load(deps.storage, U32Key::new(index)) {
            Ok(asset_reference_vec) => asset_reference_vec,
            Err(_) => {
                return Err(StdError::generic_err(format!(
                    "no market reference exists with index: {}",
                    index
                )))
            }
        };

    match MARKETS.load(deps.storage, asset_reference_vec.as_slice()) {
        Ok(asset_market) => Ok((asset_reference_vec, asset_market)),
        Err(_) => Err(StdError::generic_err(format!(
            "no asset market exists with asset reference: {}",
            String::from_utf8(asset_reference_vec).expect("Found invalid UTF-8")
        ))),
    }
}

// bitwise operations
/// Gets bit: true: 1, false: 0
pub fn get_bit(bitmap: Uint128, index: u32) -> StdResult<bool> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    Ok(((bitmap.u128() >> index) & 1) == 1)
}

/// Sets bit to 1
fn set_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() | (1 << index));
    Ok(())
}

/// Sets bit to 0
fn unset_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() & !(1 << index));
    Ok(())
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, coin, from_binary, BankMsg, OwnedDeps, SubMsg};

    use mars_core::tax::deduct_tax;
    use mars_core::testing::{
        mock_dependencies, mock_env, mock_env_at_block_time, mock_info, MarsMockQuerier,
        MockEnvParams,
    };

    use crate::interest_rate_models::{
        get_liquidity_rate, linear_get_borrow_rate, update_market_interest_rates_with_model,
        DynamicInterestRateModelParams, DynamicInterestRateModelState, InterestRateModel,
        InterestRateModelError, InterestRateModelParams, LinearInterestRateModelParams,
    };
    use crate::interest_rates::{
        calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
        get_updated_borrow_index, get_updated_liquidity_index, ScalingOperation, SCALING_FACTOR,
    };
    use crate::msg::CreateOrUpdateConfig;
    use crate::MarketError;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env(MockEnvParams::default());

        // Config with base params valid (just update the rest)
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(10u64),
            close_factor: None,
        };

        // *
        // init config with empty params
        // *
        let empty_config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            ma_token_code_id: None,
            close_factor: None,
        };
        let msg = InstantiateMsg {
            config: empty_config,
        };
        let info = mock_info("owner");
        let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());

        // *
        // init config with close_factor greater than 1
        // *
        let mut close_factor = Decimal::from_ratio(13u128, 10u128);
        let config = CreateOrUpdateConfig {
            close_factor: Some(close_factor),
            ..base_config.clone()
        };
        let msg = InstantiateMsg { config };
        let info = mock_info("owner");
        let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            MarsError::InvalidParam {
                param_name: "close_factor".to_string(),
                invalid_value: "1.3".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );

        // *
        // init config with valid params
        // *
        close_factor = Decimal::from_ratio(1u128, 2u128);
        let config = CreateOrUpdateConfig {
            close_factor: Some(close_factor),
            ..base_config
        };
        let msg = InstantiateMsg { config };

        // we can just call .unwrap() to assert this was a success
        let info = mock_info("owner");
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.ma_token_code_id);
        assert_eq!(0, value.market_count);
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env(MockEnvParams::default());

        // *
        // init config with valid params
        // *
        let mut close_factor = Decimal::from_ratio(1u128, 4u128);
        let init_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(20u64),
            close_factor: Some(close_factor),
        };
        let msg = InstantiateMsg {
            config: init_config.clone(),
        };
        // we can just call .unwrap() to assert this was a success
        let info = mock_info("owner");
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // *
        // non owner is not authorized
        // *
        let msg = ExecuteMsg::UpdateConfig {
            config: init_config.clone(),
        };
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // update config with close_factor
        // *
        close_factor = Decimal::from_ratio(13u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            close_factor: Some(close_factor),
            ..init_config.clone()
        };
        let msg = ExecuteMsg::UpdateConfig { config };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            MarsError::InvalidParam {
                param_name: "close_factor".to_string(),
                invalid_value: "1.3".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );

        // *
        // update config with all new params
        // *
        close_factor = Decimal::from_ratio(1u128, 20u128);
        let config = CreateOrUpdateConfig {
            owner: Some("new_owner".to_string()),
            address_provider_address: Some("new_address_provider".to_string()),
            ma_token_code_id: Some(40u64),
            close_factor: Some(close_factor),
        };
        let msg = ExecuteMsg::UpdateConfig {
            config: config.clone(),
        };

        // we can just call .unwrap() to assert this was a success
        let info = mock_info("owner");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(&deps.storage).unwrap();

        assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
        assert_eq!(
            new_config.address_provider_address,
            Addr::unchecked(config.address_provider_address.unwrap())
        );
        assert_eq!(
            new_config.ma_token_code_id,
            config.ma_token_code_id.unwrap()
        );
        assert_eq!(new_config.close_factor, config.close_factor.unwrap());
    }

    #[test]
    fn test_init_asset() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env(MockEnvParams::default());

        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(5u64),
            close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
        };
        let msg = InstantiateMsg { config };
        let info = mock_info("owner");
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
            max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
            kp_1: Decimal::from_ratio(3u128, 1u128),
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
            kp_2: Decimal::from_ratio(2u128, 1u128),
            update_threshold_seconds: 1,
            update_threshold_txs: 1,
        };
        let asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
            max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
            reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
            liquidation_threshold: Some(Decimal::one()),
            liquidation_bonus: Some(Decimal::zero()),
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                dynamic_ir_params.clone(),
            )),
            active: Some(true),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
        };
        let cw20_addr = Addr::unchecked("otherasset");

        // non owner is not authorized
        {
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
                asset_symbol: None,
            };
            let info = mock_info("somebody");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, MarsError::Unauthorized {}.into());
        }

        // init asset with empty params
        {
            let empty_asset_params = InitOrUpdateAssetParams {
                max_loan_to_value: None,
                liquidation_threshold: None,
                liquidation_bonus: None,
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: empty_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());
        }

        // init asset with max_loan_to_value greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "max_loan_to_value".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // init asset with liquidation_threshold greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "liquidation_threshold".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // init asset with liquidation_bonus greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "liquidation_bonus".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // init asset where LTV >= liquidity threshold
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                max_loan_to_value: Some(Decimal::from_ratio(5u128, 10u128)),
                liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(MarketError::InvalidLiquidationThreshold {
                    liquidation_threshold: Decimal::from_ratio(1u128, 2u128),
                    max_loan_to_value: Decimal::from_ratio(1u128, 2u128)
                })
            );
        }

        // init asset where min borrow rate >= max borrow rate
        {
            let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
                min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                max_borrow_rate: Decimal::from_ratio(4u128, 10u128),
                ..dynamic_ir_params.clone()
            };
            let invalid_asset_params = InitOrUpdateAssetParams {
                interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                    invalid_dynamic_ir_params,
                )),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InterestRateModel(InterestRateModelError::InvalidMinMaxBorrowRate {
                    min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                    max_borrow_rate: Decimal::from_ratio(4u128, 10u128)
                })
            );
        }

        // init asset where optimal utilization rate > 1
        {
            let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
                optimal_utilization_rate: Decimal::from_ratio(11u128, 10u128),
                ..dynamic_ir_params.clone()
            };
            let invalid_asset_params = InitOrUpdateAssetParams {
                interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                    invalid_dynamic_ir_params,
                )),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InterestRateModel(
                    InterestRateModelError::InvalidOptimalUtilizationRate {}
                )
            );
        }

        // owner is authorized
        {
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            // should have asset market with Canonical default address
            let market = MARKETS.load(&deps.storage, b"someasset").unwrap();
            assert_eq!(zero_address(), market.ma_token_address);
            // should have 0 index
            assert_eq!(0, market.index);
            // should have asset_type Native
            assert_eq!(AssetType::Native, market.asset_type);

            // should store reference in market index
            let market_reference = MARKET_REFERENCES_BY_INDEX
                .load(&deps.storage, U32Key::new(0))
                .unwrap();
            assert_eq!(b"someasset", market_reference.as_slice());

            // Should have market count of 1
            let money_market = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(money_market.market_count, 1);

            // should instantiate a liquidity token
            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: Some("protocol_admin".to_string()),
                    code_id: 5u64,
                    msg: to_binary(&ma_token::msg::InstantiateMsg {
                        name: String::from("Mars someasset Liquidity Token"),
                        symbol: String::from("masomeasset"),
                        decimals: 6,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: MOCK_CONTRACT_ADDR.to_string(),
                            cap: None,
                        }),
                        init_hook: Some(ma_token::msg::InitHook {
                            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                            msg: to_binary(&ExecuteMsg::InitAssetTokenCallback {
                                reference: market_reference,
                            })
                            .unwrap()
                        }),
                        marketing: Some(InstantiateMarketingInfo {
                            project: Some("Mars Protocol".to_string()),
                            description: Some(
                                "Interest earning token representing deposits for someasset"
                                    .to_string()
                            ),

                            marketing: Some("protocol_admin".to_string()),
                            logo: None,
                        }),
                        red_bank_address: MOCK_CONTRACT_ADDR.to_string(),
                        incentives_address: "incentives".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                    label: "masomeasset".to_string()
                })),]
            );

            assert_eq!(
                res.attributes,
                vec![attr("action", "init_asset"), attr("asset", "someasset"),],
            );
        }

        // can't init more than once
        {
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, ContractError::AssetAlreadyInitialized {});
        }

        // callback comes back with created token
        {
            let msg = ExecuteMsg::InitAssetTokenCallback {
                reference: "someasset".into(),
            };
            let info = mock_info("mtokencontract");
            execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            // should have asset market with contract address
            let market = MARKETS.load(&deps.storage, b"someasset").unwrap();
            assert_eq!(Addr::unchecked("mtokencontract"), market.ma_token_address);
            assert_eq!(Decimal::one(), market.liquidity_index);
        }

        // calling this again should not be allowed
        {
            let msg = ExecuteMsg::InitAssetTokenCallback {
                reference: "someasset".into(),
            };
            let info = mock_info("mtokencontract");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, MarsError::Unauthorized {}.into());
        }

        // Initialize a cw20 asset
        {
            deps.querier
                .set_cw20_symbol(cw20_addr.clone(), "otherasset".to_string());
            let info = mock_info("owner");

            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Cw20 {
                    contract_addr: cw20_addr.to_string(),
                },
                asset_params,
                asset_symbol: None,
            };
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            let market = MARKETS.load(&deps.storage, cw20_addr.as_bytes()).unwrap();
            // should have asset market with Canonical default address
            assert_eq!(zero_address(), market.ma_token_address);
            // should have index 1
            assert_eq!(1, market.index);
            // should have asset_type Cw20
            assert_eq!(AssetType::Cw20, market.asset_type);

            // should store reference in market index
            let market_reference = MARKET_REFERENCES_BY_INDEX
                .load(&deps.storage, U32Key::new(1))
                .unwrap();
            assert_eq!(cw20_addr.as_bytes(), market_reference.as_slice());

            // should have an asset_type of cw20
            assert_eq!(AssetType::Cw20, market.asset_type);

            // Should have market count of 2
            let money_market = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(2, money_market.market_count);

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "init_asset"),
                    attr("asset", cw20_addr.clone())
                ],
            );
        }

        // cw20 callback comes back with created token
        {
            let msg = ExecuteMsg::InitAssetTokenCallback {
                reference: Vec::from(cw20_addr.as_bytes()),
            };
            let info = mock_info("mtokencontract");
            execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            // should have asset market with contract address
            let market = MARKETS.load(&deps.storage, cw20_addr.as_bytes()).unwrap();
            assert_eq!(Addr::unchecked("mtokencontract"), market.ma_token_address);
            assert_eq!(Decimal::one(), market.liquidity_index);
        }

        // calling this again should not be allowed
        {
            let msg = ExecuteMsg::InitAssetTokenCallback {
                reference: Vec::from(cw20_addr.as_bytes()),
            };
            let info = mock_info("mtokencontract");
            let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(error_res, MarsError::Unauthorized {}.into());
        }
    }

    #[test]
    fn test_init_asset_with_msg_symbol() {
        let mut deps = th_setup(&[]);
        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
            max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
            kp_1: Decimal::from_ratio(3u128, 1u128),
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
            kp_2: Decimal::from_ratio(2u128, 1u128),
            update_threshold_seconds: 1,
            update_threshold_txs: 1,
        };
        let asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
            max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
            reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
            liquidation_threshold: Some(Decimal::one()),
            liquidation_bonus: Some(Decimal::zero()),
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                dynamic_ir_params.clone(),
            )),
            active: Some(true),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
        };
        let msg = ExecuteMsg::InitAsset {
            asset: Asset::Native {
                denom: "someasset".to_string(),
            },
            asset_params: asset_params.clone(),
            asset_symbol: Some("COIN".to_string()),
        };
        let info = mock_info("owner");
        let env = mock_env(MockEnvParams::default());
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // should instantiate a liquidity token
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("protocol_admin".to_string()),
                code_id: 1u64,
                msg: to_binary(&ma_token::msg::InstantiateMsg {
                    name: String::from("Mars COIN Liquidity Token"),
                    symbol: String::from("maCOIN"),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                    init_hook: Some(ma_token::msg::InitHook {
                        contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                        msg: to_binary(&ExecuteMsg::InitAssetTokenCallback {
                            reference: b"someasset".to_vec(),
                        })
                        .unwrap()
                    }),
                    marketing: Some(InstantiateMarketingInfo {
                        project: Some("Mars Protocol".to_string()),
                        description: Some(
                            "Interest earning token representing deposits for COIN".to_string()
                        ),

                        marketing: Some("protocol_admin".to_string()),
                        logo: None,
                    }),
                    red_bank_address: MOCK_CONTRACT_ADDR.to_string(),
                    incentives_address: "incentives".to_string(),
                })
                .unwrap(),
                funds: vec![],
                label: "maCOIN".to_string()
            })),]
        );
    }

    #[test]
    fn test_update_asset() {
        let mut deps = mock_dependencies(&[]);
        let start_time = 100000000;
        let env = mock_env_at_block_time(start_time);

        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(5u64),
            close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
        };
        let msg = InstantiateMsg { config };
        let info = mock_info("owner");
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
            max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
            kp_1: Decimal::from_ratio(3u128, 1u128),
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
            kp_2: Decimal::from_ratio(2u128, 1u128),

            update_threshold_txs: 1,
            update_threshold_seconds: 1,
        };

        let asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
            max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                dynamic_ir_params.clone(),
            )),
            active: Some(true),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
        };

        // non owner is not authorized
        {
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
            };
            let info = mock_info("somebody");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, MarsError::Unauthorized {}.into());
        }

        // owner is authorized but can't update asset if not initialized first
        {
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(error_res, ContractError::AssetNotInitialized {});
        }

        // initialize asset
        {
            let msg = ExecuteMsg::InitAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
                asset_symbol: None,
            };
            let info = mock_info("owner");
            let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        }

        // update asset with max_loan_to_value greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "max_loan_to_value".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // update asset with liquidation_threshold greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "liquidation_threshold".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // update asset with liquidation_bonus greater than 1
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
                ..asset_params.clone()
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(
                    MarsError::InvalidParam {
                        param_name: "liquidation_bonus".to_string(),
                        invalid_value: "1.1".to_string(),
                        predicate: "<= 1".to_string(),
                    }
                    .into()
                )
            );
        }

        // update asset where LTV >= liquidity threshold
        {
            let invalid_asset_params = InitOrUpdateAssetParams {
                max_loan_to_value: Some(Decimal::from_ratio(6u128, 10u128)),
                liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
                ..asset_params
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::Market(MarketError::InvalidLiquidationThreshold {
                    liquidation_threshold: Decimal::from_ratio(1u128, 2u128),
                    max_loan_to_value: Decimal::from_ratio(6u128, 10u128)
                })
            );
        }

        // update asset where min borrow rate >= max borrow rate
        {
            let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
                min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                max_borrow_rate: Decimal::from_ratio(4u128, 10u128),
                ..dynamic_ir_params
            };
            let invalid_asset_params = InitOrUpdateAssetParams {
                interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                    invalid_dynamic_ir_params.clone(),
                )),
                ..asset_params
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InterestRateModel(InterestRateModelError::InvalidMinMaxBorrowRate {
                    min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                    max_borrow_rate: Decimal::from_ratio(4u128, 10u128)
                })
            );
        }

        // update asset where optimal utilization rate > 1
        {
            let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
                optimal_utilization_rate: Decimal::from_ratio(11u128, 10u128),
                ..dynamic_ir_params
            };
            let invalid_asset_params = InitOrUpdateAssetParams {
                interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                    invalid_dynamic_ir_params.clone(),
                )),
                ..asset_params
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: invalid_asset_params,
            };
            let info = mock_info("owner");
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InterestRateModel(
                    InterestRateModelError::InvalidOptimalUtilizationRate {}
                )
            );
        }

        // update asset with new params
        {
            let dynamic_ir_params = DynamicInterestRateModelParams {
                min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
                max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
                kp_1: Decimal::from_ratio(3u128, 1u128),
                optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
                kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
                kp_2: Decimal::from_ratio(2u128, 1u128),
                update_threshold_txs: 1,
                update_threshold_seconds: 1,
            };
            let asset_params = InitOrUpdateAssetParams {
                initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
                max_loan_to_value: Some(Decimal::from_ratio(60u128, 100u128)),
                reserve_factor: Some(Decimal::from_ratio(10u128, 100u128)),
                liquidation_threshold: Some(Decimal::from_ratio(90u128, 100u128)),
                liquidation_bonus: Some(Decimal::from_ratio(12u128, 100u128)),
                interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                    dynamic_ir_params.clone(),
                )),
                active: Some(true),
                deposit_enabled: Some(true),
                borrow_enabled: Some(true),
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: asset_params.clone(),
            };
            let info = mock_info("owner");
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            let new_market = MARKETS.load(&deps.storage, b"someasset").unwrap();
            assert_eq!(0, new_market.index);
            assert_eq!(
                asset_params.max_loan_to_value.unwrap(),
                new_market.max_loan_to_value
            );
            assert_eq!(
                asset_params.reserve_factor.unwrap(),
                new_market.reserve_factor
            );
            assert_eq!(
                asset_params.liquidation_threshold.unwrap(),
                new_market.liquidation_threshold
            );
            assert_eq!(
                asset_params.liquidation_bonus.unwrap(),
                new_market.liquidation_bonus
            );
            assert_eq!(
                InterestRateModel::Dynamic {
                    params: dynamic_ir_params,
                    state: DynamicInterestRateModelState {
                        txs_since_last_borrow_rate_update: 1,
                        borrow_rate_last_updated: env.block.time.seconds(),
                    }
                },
                new_market.interest_rate_model
            );

            let new_market_reference = MARKET_REFERENCES_BY_INDEX
                .load(&deps.storage, U32Key::new(0))
                .unwrap();
            assert_eq!(b"someasset", new_market_reference.as_slice());

            let new_money_market = GLOBAL_STATE.load(&deps.storage).unwrap();
            assert_eq!(new_money_market.market_count, 1);

            assert_eq!(res.messages, vec![],);

            assert_eq!(
                res.attributes,
                vec![attr("action", "update_asset"), attr("asset", "someasset"),],
            );
        }

        // update asset with empty params
        {
            let market_before = MARKETS.load(&deps.storage, b"someasset").unwrap();

            let empty_asset_params = InitOrUpdateAssetParams {
                initial_borrow_rate: None,
                max_loan_to_value: None,
                reserve_factor: None,
                liquidation_threshold: None,
                liquidation_bonus: None,
                interest_rate_model_params: None,
                active: None,
                deposit_enabled: None,
                borrow_enabled: None,
            };
            let msg = ExecuteMsg::UpdateAsset {
                asset: Asset::Native {
                    denom: "someasset".to_string(),
                },
                asset_params: empty_asset_params,
            };
            let info = mock_info("owner");
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            // no interest updated event
            assert_eq!(res.events.len(), 0);

            let new_market = MARKETS.load(&deps.storage, b"someasset").unwrap();
            // should keep old params
            assert_eq!(0, new_market.index);
            assert_eq!(market_before.borrow_rate, new_market.borrow_rate);
            assert_eq!(
                market_before.max_loan_to_value,
                new_market.max_loan_to_value
            );
            assert_eq!(market_before.reserve_factor, new_market.reserve_factor);
            assert_eq!(
                market_before.liquidation_threshold,
                new_market.liquidation_threshold
            );
            assert_eq!(
                market_before.liquidation_bonus,
                new_market.liquidation_bonus
            );
            if let InterestRateModel::Dynamic {
                params: market_dynamic_ir_params,
                state: market_dynamic_ir_state,
            } = new_market.interest_rate_model
            {
                assert_eq!(
                    dynamic_ir_params.min_borrow_rate,
                    market_dynamic_ir_params.min_borrow_rate
                );
                assert_eq!(
                    dynamic_ir_params.max_borrow_rate,
                    market_dynamic_ir_params.max_borrow_rate
                );
                assert_eq!(dynamic_ir_params.kp_1, market_dynamic_ir_params.kp_1);
                assert_eq!(
                    dynamic_ir_params.kp_augmentation_threshold,
                    market_dynamic_ir_params.kp_augmentation_threshold
                );
                assert_eq!(dynamic_ir_params.kp_2, market_dynamic_ir_params.kp_2);
                assert_eq!(
                    dynamic_ir_params.update_threshold_txs,
                    market_dynamic_ir_params.update_threshold_txs
                );
                assert_eq!(
                    dynamic_ir_params.update_threshold_seconds,
                    market_dynamic_ir_params.update_threshold_seconds
                );

                assert_eq!(1, market_dynamic_ir_state.txs_since_last_borrow_rate_update);
                assert_eq!(
                    env.block.time.seconds(),
                    market_dynamic_ir_state.borrow_rate_last_updated
                );
            } else {
                panic!("INCORRECT STRATEGY")
            }
        }
    }

    #[test]
    fn test_update_asset_with_new_interest_rate_model_params() {
        let mut deps = mock_dependencies(&[]);

        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(5u64),
            close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
        };
        let msg = InstantiateMsg { config };
        let info = mock_info("owner");
        let env = mock_env(MockEnvParams::default());
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(10u128, 100u128),
            max_borrow_rate: Decimal::from_ratio(60u128, 100u128),
            kp_1: Decimal::from_ratio(4u128, 1u128),
            optimal_utilization_rate: Decimal::from_ratio(90u128, 100u128),
            kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
            kp_2: Decimal::from_ratio(3u128, 1u128),
            update_threshold_txs: 1,
            update_threshold_seconds: 1,
        };

        let asset_params_with_dynamic_ir = InitOrUpdateAssetParams {
            initial_borrow_rate: Some(Decimal::from_ratio(15u128, 100u128)),
            max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(2u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                dynamic_ir_params.clone(),
            )),
            active: Some(true),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
        };

        let msg = ExecuteMsg::InitAsset {
            asset: Asset::Native {
                denom: "someasset".to_string(),
            },
            asset_params: asset_params_with_dynamic_ir.clone(),
            asset_symbol: None,
        };
        let info = mock_info("owner");
        let env = mock_env_at_block_time(1_000_000);
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Verify if IR model is saved correctly
        let market_before = MARKETS.load(&deps.storage, b"someasset").unwrap();
        assert_eq!(
            market_before.interest_rate_model,
            InterestRateModel::Dynamic {
                params: dynamic_ir_params,
                state: DynamicInterestRateModelState {
                    txs_since_last_borrow_rate_update: 0,
                    borrow_rate_last_updated: 1_000_000
                }
            }
        );

        let linear_ir_params = LinearInterestRateModelParams {
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            base: Decimal::from_ratio(0u128, 100u128),
            slope_1: Decimal::from_ratio(8u128, 100u128),
            slope_2: Decimal::from_ratio(48u128, 100u128),
        };
        let asset_params_with_linear_ir = InitOrUpdateAssetParams {
            interest_rate_model_params: Some(InterestRateModelParams::Linear(
                linear_ir_params.clone(),
            )),
            ..asset_params_with_dynamic_ir
        };
        let msg = ExecuteMsg::UpdateAsset {
            asset: Asset::Native {
                denom: "someasset".to_string(),
            },
            asset_params: asset_params_with_linear_ir.clone(),
        };
        let info = mock_info("owner");
        let env = mock_env_at_block_time(2_000_000);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Verify if IR model is updated
        let new_market = MARKETS.load(&deps.storage, b"someasset").unwrap();
        assert_eq!(
            new_market.interest_rate_model,
            InterestRateModel::Linear {
                params: linear_ir_params.clone()
            }
        );

        // Indices should have been updated using previous interest rate
        let expected_indices = th_get_expected_indices(&market_before, 2_000_000);
        assert_eq!(new_market.liquidity_index, expected_indices.liquidity);
        assert_eq!(new_market.borrow_index, expected_indices.borrow);
        assert_eq!(new_market.indexes_last_updated, 2_000_000);

        // Interest rate should have been recomputed using new strategy and values
        let expected_borrow_rate =
            linear_get_borrow_rate(&linear_ir_params, Decimal::zero()).unwrap();
        let expected_liquidity_rate = Decimal::zero(); // zero utilization rate
        assert_eq!(new_market.borrow_rate, expected_borrow_rate);
        assert_eq!(new_market.liquidity_rate, expected_liquidity_rate);

        // proper event is logged
        assert_eq!(
            res.events,
            vec![Event::new("interests_updated")
                .add_attribute("asset", "someasset")
                .add_attribute("borrow_index", new_market.borrow_index.to_string())
                .add_attribute("liquidity_index", new_market.liquidity_index.to_string())
                .add_attribute("borrow_rate", expected_borrow_rate.to_string())
                .add_attribute("liquidity_rate", expected_liquidity_rate.to_string())]
        );

        // mint message is not sent as debt is 0
        assert_eq!(res.messages, vec![])
    }

    #[test]
    fn test_update_asset_new_reserve_factor_accrues_interest_rate() {
        let asset_liquidity = 10_000_000_000000_u128;
        let mut deps = th_setup(&[coin(asset_liquidity, "somecoin")]);

        let ma_token_address = Addr::unchecked("ma_token");

        let linear_ir_model_params = LinearInterestRateModelParams {
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            base: Decimal::zero(),
            slope_1: Decimal::from_ratio(1_u128, 2_u128),
            slope_2: Decimal::from_ratio(2_u128, 1_u128),
        };
        let linear_ir_model = InterestRateModel::Linear {
            params: linear_ir_model_params.clone(),
        };

        let asset_initial_debt = Uint128::new(2_000_000_000000);
        let market_before = th_init_market(
            deps.as_mut(),
            b"somecoin",
            &Market {
                reserve_factor: Decimal::from_ratio(1_u128, 10_u128),
                borrow_index: Decimal::one(),
                liquidity_index: Decimal::one(),
                indexes_last_updated: 1_000_000,
                borrow_rate: Decimal::from_ratio(12u128, 100u128),
                liquidity_rate: Decimal::from_ratio(12u128, 100u128),
                debt_total_scaled: compute_scaled_amount(
                    asset_initial_debt,
                    Decimal::one(),
                    ScalingOperation::Debt,
                )
                .unwrap(),
                ma_token_address: ma_token_address,
                interest_rate_model: linear_ir_model.clone(),
                ..Default::default()
            },
        );

        let asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: None,
            max_loan_to_value: None,
            reserve_factor: Some(Decimal::from_ratio(2_u128, 10_u128)),
            liquidation_threshold: None,
            liquidation_bonus: None,
            interest_rate_model_params: None,
            active: None,
            deposit_enabled: None,
            borrow_enabled: None,
        };
        let msg = ExecuteMsg::UpdateAsset {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            asset_params: asset_params,
        };
        let info = mock_info("owner");
        let env = mock_env_at_block_time(1_500_000);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let new_market = MARKETS.load(&deps.storage, b"somecoin").unwrap();

        // Indices should have been updated using previous interest rate
        let expected_indices = th_get_expected_indices(&market_before, 1_500_000);
        assert_eq!(new_market.liquidity_index, expected_indices.liquidity);
        assert_eq!(new_market.borrow_index, expected_indices.borrow);
        assert_eq!(new_market.indexes_last_updated, 1_500_000);

        // Interest rate should have been recomputed using new strategy and values
        let expected_debt =
            compute_underlying_amount(new_market.debt_total_scaled, new_market.borrow_index)
                .unwrap();
        let expected_liquidity = Uint128::new(asset_liquidity);
        let expected_utilization_rate =
            Decimal::from_ratio(expected_debt, expected_liquidity + expected_debt);

        let expected_borrow_rate =
            linear_get_borrow_rate(&linear_ir_model_params, expected_utilization_rate).unwrap();

        let expected_liquidity_rate = get_liquidity_rate(
            expected_borrow_rate,
            expected_utilization_rate,
            new_market.reserve_factor,
        )
        .unwrap();

        assert_eq!(new_market.borrow_rate, expected_borrow_rate);
        assert_eq!(new_market.liquidity_rate, expected_liquidity_rate);

        // proper event is logged
        assert_eq!(
            res.events,
            vec![Event::new("interests_updated")
                .add_attribute("asset", "somecoin")
                .add_attribute("borrow_index", new_market.borrow_index.to_string())
                .add_attribute("liquidity_index", new_market.liquidity_index.to_string())
                .add_attribute("borrow_rate", expected_borrow_rate.to_string())
                .add_attribute("liquidity_rate", expected_liquidity_rate.to_string())]
        );

        let current_debt_total =
            compute_underlying_amount(new_market.debt_total_scaled, new_market.borrow_index)
                .unwrap();
        let interest_accrued = current_debt_total - asset_initial_debt;
        let expected_protocol_rewards = interest_accrued * market_before.reserve_factor;
        // mint message is sent because debt is non zero
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: market_before.ma_token_address.to_string(),
                msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                    recipient: "protocol_rewards_collector".to_string(),
                    amount: compute_scaled_amount(
                        expected_protocol_rewards,
                        new_market.liquidity_index,
                        ScalingOperation::Liquidity
                    )
                    .unwrap()
                })
                .unwrap(),
                funds: vec![]
            })),]
        );
    }

    #[test]
    fn test_init_asset_callback_cannot_be_called_on_its_own() {
        let mut deps = th_setup(&[]);

        let env = mock_env(MockEnvParams::default());
        let info = mock_info("mtokencontract");
        let msg = ExecuteMsg::InitAssetTokenCallback {
            reference: "uluna".into(),
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            StdError::not_found("mars_core::red_bank::Market").into()
        );
    }

    #[test]
    fn test_deposit_native_asset() {
        let initial_liquidity = 10000000;
        let mut deps = th_setup(&[coin(initial_liquidity, "somecoin")]);
        let reserve_factor = Decimal::from_ratio(1u128, 10u128);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::from_ratio(11u128, 10u128),
            max_loan_to_value: Decimal::one(),
            borrow_index: Decimal::from_ratio(1u128, 1u128),
            borrow_rate: Decimal::from_ratio(10u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor,
            debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
            indexes_last_updated: 10000000,
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let deposit_amount = 110000;
        let env = mock_env_at_block_time(10000100);
        let info =
            cosmwasm_std::testing::mock_info("depositor", &[coin(deposit_amount, "somecoin")]);
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: None,
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market,
            env.block.time.seconds(),
            initial_liquidity,
            Default::default(),
        );

        let expected_mint_amount = compute_scaled_amount(
            Uint128::from(deposit_amount),
            expected_params.liquidity_index,
            ScalingOperation::Liquidity,
        )
        .unwrap();

        // mints coin_amount/liquidity_index
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "depositor".to_string(),
                    amount: expected_mint_amount.into(),
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit"),
                attr("asset", "somecoin"),
                attr("sender", "depositor"),
                attr("user", "depositor"),
                attr("amount", deposit_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event("somecoin", true, "depositor".to_string()),
                th_build_interests_updated_event("somecoin", &expected_params)
            ]
        );

        let market = MARKETS.load(&deps.storage, b"somecoin").unwrap();
        assert_eq!(market.borrow_rate, expected_params.borrow_rate);
        assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
        assert_eq!(market.liquidity_index, expected_params.liquidity_index);
        assert_eq!(market.borrow_index, expected_params.borrow_index);

        // send many native coins
        let info = cosmwasm_std::testing::mock_info(
            "depositor",
            &[coin(100, "somecoin1"), coin(200, "somecoin2")],
        );
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin2"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InvalidNativeCoinsSent {
                denom: "somecoin2".to_string()
            }
        );

        // empty deposit fails
        let info = mock_info("depositor");
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InvalidNativeCoinsSent {
                denom: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_deposit_cw20() {
        let initial_liquidity = 10_000_000;
        let mut deps = th_setup(&[]);

        let cw20_addr = Addr::unchecked("somecontract");

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::from_ratio(11u128, 10u128),
            max_loan_to_value: Decimal::one(),
            borrow_index: Decimal::from_ratio(1u128, 1u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(4u128, 100u128),
            debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
            indexes_last_updated: 10_000_000,
            asset_type: AssetType::Cw20,
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), cw20_addr.as_bytes(), &mock_market);

        // set initial balance on cw20 contract
        deps.querier.set_cw20_balances(
            cw20_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                initial_liquidity.into(),
            )],
        );
        // set symbol for cw20 contract
        deps.querier
            .set_cw20_symbol(cw20_addr.clone(), "somecoin".to_string());

        let deposit_amount = 110000u128;
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            msg: to_binary(&ReceiveMsg::DepositCw20 { on_behalf_of: None }).unwrap(),
            sender: "depositor".to_string(),
            amount: Uint128::new(deposit_amount),
        });
        let env = mock_env_at_block_time(10000100);
        let info =
            cosmwasm_std::testing::mock_info("somecontract", &[coin(deposit_amount, "somecoin")]);

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market,
            env.block.time.seconds(),
            initial_liquidity,
            Default::default(),
        );

        let expected_mint_amount = compute_scaled_amount(
            Uint128::from(deposit_amount),
            expected_params.liquidity_index,
            ScalingOperation::Liquidity,
        )
        .unwrap();

        // No rewards to distribute so no mint message
        assert_eq!(
            expected_params.protocol_rewards_to_distribute,
            Uint128::zero()
        );
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "depositor".to_string(),
                    amount: expected_mint_amount.into(),
                })
                .unwrap(),
                funds: vec![]
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit"),
                attr("asset", cw20_addr.clone()),
                attr("sender", "depositor"),
                attr("user", "depositor"),
                attr("amount", deposit_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    cw20_addr.as_str(),
                    true,
                    "depositor".to_string()
                ),
                th_build_interests_updated_event(cw20_addr.as_str(), &expected_params)
            ]
        );

        // empty deposit fails
        let info = mock_info("depositor");
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            msg: to_binary(&ReceiveMsg::DepositCw20 { on_behalf_of: None }).unwrap(),
            sender: "depositor".to_string(),
            amount: Uint128::new(deposit_amount),
        });
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            StdError::not_found("mars_core::red_bank::Market").into()
        );
    }

    #[test]
    fn test_cannot_deposit_if_no_market() {
        let mut deps = th_setup(&[]);
        let env = mock_env(MockEnvParams::default());

        let info = cosmwasm_std::testing::mock_info("depositer", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            StdError::not_found("mars_core::red_bank::Market").into()
        );
    }

    #[test]
    fn test_cannot_deposit_if_market_not_active() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: false,
            deposit_enabled: true,
            ..Default::default()
        };
        th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        // Check error when deposit not allowed on market
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("depositor", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::MarketNotActive {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_cannot_deposit_if_market_not_enabled() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: true,
            deposit_enabled: false,
            ..Default::default()
        };
        th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        // Check error when deposit not allowed on market
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("depositor", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::DepositNotEnabled {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_deposit_on_behalf_of() {
        let initial_liquidity = 10000000;
        let mut deps = th_setup(&[coin(initial_liquidity, "somecoin")]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let depositor_addr = Addr::unchecked("depositor");
        let another_user_addr = Addr::unchecked("another_user");
        let deposit_amount = 110000;
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info(
            depositor_addr.as_str(),
            &[coin(deposit_amount, "somecoin")],
        );
        let msg = ExecuteMsg::DepositNative {
            denom: String::from("somecoin"),
            on_behalf_of: Some(another_user_addr.to_string()),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let expected_mint_amount = compute_scaled_amount(
            Uint128::from(deposit_amount),
            market.liquidity_index,
            ScalingOperation::Liquidity,
        )
        .unwrap();

        // 'depositor' should not be saved
        let _user = USERS.load(&deps.storage, &depositor_addr).unwrap_err();

        // 'another_user' should have collateral bit set
        let user = USERS.load(&deps.storage, &another_user_addr).unwrap();
        assert!(get_bit(user.collateral_assets, market.index).unwrap());

        // recipient should be `another_user`
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "matoken".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: another_user_addr.to_string(),
                    amount: expected_mint_amount.into(),
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit"),
                attr("asset", "somecoin"),
                attr("sender", depositor_addr),
                attr("user", another_user_addr),
                attr("amount", deposit_amount.to_string()),
            ]
        );
    }

    #[test]
    fn test_withdraw_native() {
        // Withdraw native token
        let initial_available_liquidity = 12000000u128;
        let mut deps = th_setup(&[coin(initial_available_liquidity, "somecoin")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("somecoin"), Uint128::new(100u128))],
        );

        let initial_liquidity_index = Decimal::from_ratio(15u128, 10u128);
        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: initial_liquidity_index,
            borrow_index: Decimal::from_ratio(2u128, 1u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(1u128, 10u128),

            debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
            indexes_last_updated: 10000000,
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let withdraw_amount = Uint128::from(20000u128);
        let seconds_elapsed = 2000u64;

        deps.querier.set_cw20_balances(
            Addr::unchecked("matoken"),
            &[(
                Addr::unchecked("withdrawer"),
                Uint128::new(2_000_000) * SCALING_FACTOR,
            )],
        );

        let market_initial = th_init_market(deps.as_mut(), b"somecoin", &mock_market);
        MARKET_REFERENCES_BY_MA_TOKEN
            .save(
                deps.as_mut().storage,
                &Addr::unchecked("matoken"),
                &(b"somecoin".to_vec()),
            )
            .unwrap();

        let withdrawer_addr = Addr::unchecked("withdrawer");
        let user = User::default();
        USERS
            .save(deps.as_mut().storage, &withdrawer_addr, &user)
            .unwrap();

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: Some(withdraw_amount),
            recipient: None,
        };

        let env = mock_env_at_block_time(mock_market.indexes_last_updated + seconds_elapsed);
        let info = mock_info("withdrawer");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let market = MARKETS.load(&deps.storage, b"somecoin").unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market_initial,
            mock_market.indexes_last_updated + seconds_elapsed,
            initial_available_liquidity,
            TestUtilizationDeltas {
                less_liquidity: withdraw_amount.into(),
                ..Default::default()
            },
        );

        let withdraw_amount_scaled = compute_scaled_amount(
            withdraw_amount,
            expected_params.liquidity_index,
            ScalingOperation::Liquidity,
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "matoken".to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_params.protocol_rewards_to_distribute,
                            market.liquidity_index,
                            ScalingOperation::Liquidity
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "matoken".to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                        user: withdrawer_addr.to_string(),
                        amount: withdraw_amount_scaled.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: withdrawer_addr.to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: String::from("somecoin"),
                            amount: withdraw_amount.into(),
                        }
                    )
                    .unwrap()],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("asset", "somecoin"),
                attr("user", "withdrawer"),
                attr("recipient", "withdrawer"),
                attr("burn_amount", withdraw_amount_scaled.to_string()),
                attr("withdraw_amount", withdraw_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event(
                "somecoin",
                &expected_params
            )]
        );

        assert_eq!(market.borrow_rate, expected_params.borrow_rate);
        assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
        assert_eq!(market.liquidity_index, expected_params.liquidity_index);
        assert_eq!(market.borrow_index, expected_params.borrow_index);
    }

    #[test]
    fn test_withdraw_cw20() {
        // Withdraw cw20 token
        let mut deps = th_setup(&[]);
        let cw20_contract_addr = Addr::unchecked("somecontract");
        let initial_available_liquidity = 12000000u128;

        let ma_token_addr = Addr::unchecked("matoken");

        deps.querier.set_cw20_balances(
            cw20_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(initial_available_liquidity),
            )],
        );
        deps.querier.set_cw20_balances(
            ma_token_addr.clone(),
            &[(
                Addr::unchecked("withdrawer"),
                Uint128::new(2_000_000) * SCALING_FACTOR,
            )],
        );

        let initial_liquidity_index = Decimal::from_ratio(15u128, 10u128);
        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: initial_liquidity_index,
            borrow_index: Decimal::from_ratio(2u128, 1u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(2u128, 100u128),
            debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
            indexes_last_updated: 10000000,
            asset_type: AssetType::Cw20,
            ..Default::default()
        };
        let withdraw_amount = Uint128::from(20000u128);
        let seconds_elapsed = 2000u64;

        let market_initial =
            th_init_market(deps.as_mut(), cw20_contract_addr.as_bytes(), &mock_market);
        MARKET_REFERENCES_BY_MA_TOKEN
            .save(
                deps.as_mut().storage,
                &ma_token_addr,
                &cw20_contract_addr.as_bytes().to_vec(),
            )
            .unwrap();

        let withdrawer_addr = Addr::unchecked("withdrawer");

        let user = User::default();
        USERS
            .save(deps.as_mut().storage, &withdrawer_addr, &user)
            .unwrap();

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Cw20 {
                contract_addr: cw20_contract_addr.to_string(),
            },
            amount: Some(withdraw_amount),
            recipient: None,
        };

        let env = mock_env_at_block_time(mock_market.indexes_last_updated + seconds_elapsed);
        let info = mock_info("withdrawer");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let market = MARKETS
            .load(&deps.storage, cw20_contract_addr.as_bytes())
            .unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market_initial,
            mock_market.indexes_last_updated + seconds_elapsed,
            initial_available_liquidity,
            TestUtilizationDeltas {
                less_liquidity: withdraw_amount.into(),
                ..Default::default()
            },
        );

        let withdraw_amount_scaled = compute_scaled_amount(
            withdraw_amount,
            expected_params.liquidity_index,
            ScalingOperation::Liquidity,
        )
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_addr.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_params.protocol_rewards_to_distribute,
                            market.liquidity_index,
                            ScalingOperation::Liquidity
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_addr.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                        user: withdrawer_addr.to_string(),
                        amount: withdraw_amount_scaled.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: withdrawer_addr.to_string(),
                        amount: withdraw_amount.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("asset", "somecontract"),
                attr("user", "withdrawer"),
                attr("recipient", "withdrawer"),
                attr("burn_amount", withdraw_amount_scaled.to_string()),
                attr("withdraw_amount", withdraw_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event(
                "somecontract",
                &expected_params
            )]
        );

        assert_eq!(market.borrow_rate, expected_params.borrow_rate);
        assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
        assert_eq!(market.liquidity_index, expected_params.liquidity_index);
        assert_eq!(market.borrow_index, expected_params.borrow_index);
    }

    #[test]
    fn test_withdraw_and_send_funds_to_another_user() {
        // Withdraw cw20 token
        let mut deps = th_setup(&[]);
        let cw20_contract_addr = Addr::unchecked("somecontract");
        let initial_available_liquidity = 12000000u128;

        let ma_token_addr = Addr::unchecked("matoken");

        let withdrawer_addr = Addr::unchecked("withdrawer");
        let another_user_addr = Addr::unchecked("another_user");

        deps.querier.set_cw20_balances(
            cw20_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(initial_available_liquidity),
            )],
        );
        let ma_token_balance_scaled = Uint128::new(2_000_000) * SCALING_FACTOR;
        deps.querier.set_cw20_balances(
            ma_token_addr.clone(),
            &[(withdrawer_addr.clone(), ma_token_balance_scaled)],
        );

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            reserve_factor: Decimal::zero(),
            asset_type: AssetType::Cw20,
            ..Default::default()
        };

        let market_initial =
            th_init_market(deps.as_mut(), cw20_contract_addr.as_bytes(), &mock_market);
        MARKET_REFERENCES_BY_MA_TOKEN
            .save(
                deps.as_mut().storage,
                &ma_token_addr,
                &cw20_contract_addr.as_bytes().to_vec(),
            )
            .unwrap();

        let user = User::default();
        USERS
            .save(deps.as_mut().storage, &withdrawer_addr, &user)
            .unwrap();

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Cw20 {
                contract_addr: cw20_contract_addr.to_string(),
            },
            amount: None,
            recipient: Some(another_user_addr.to_string()),
        };

        let env = mock_env(MockEnvParams::default());
        let info = mock_info("withdrawer");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // User should have unset bit for collateral after full withdraw
        let user = USERS.load(&deps.storage, &withdrawer_addr).unwrap();
        assert!(!get_bit(user.collateral_assets, market_initial.index).unwrap());

        let withdraw_amount =
            compute_underlying_amount(ma_token_balance_scaled, market_initial.liquidity_index)
                .unwrap();

        // Check if maToken is received by `another_user`
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ma_token_addr.to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                        user: withdrawer_addr.to_string(),
                        amount: ma_token_balance_scaled.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: another_user_addr.to_string(),
                        amount: withdraw_amount.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("asset", "somecontract"),
                attr("user", withdrawer_addr),
                attr("recipient", another_user_addr),
                attr("burn_amount", ma_token_balance_scaled.to_string()),
                attr("withdraw_amount", withdraw_amount.to_string()),
            ]
        );
    }

    #[test]
    fn test_withdraw_cannot_exceed_balance() {
        let mut deps = th_setup(&[]);
        let env = mock_env(MockEnvParams::default());

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::from_ratio(15u128, 10u128),
            ..Default::default()
        };

        deps.querier.set_cw20_balances(
            Addr::unchecked("matoken"),
            &[(Addr::unchecked("withdrawer"), Uint128::new(200u128))],
        );

        th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: Some(Uint128::from(2000u128)),
            recipient: None,
        };

        let info = mock_info("withdrawer");
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InvalidWithdrawAmount {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_cannot_withdraw_if_market_inactive() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: false,
            deposit_enabled: true,
            borrow_enabled: true,
            ..Default::default()
        };
        let _market = th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("withdrawer", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: Some(Uint128::new(2000)),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::MarketNotActive {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_withdraw_if_health_factor_not_met() {
        let initial_available_liquidity = 10000000u128;
        let mut deps = th_setup(&[coin(initial_available_liquidity, "token3")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("token3"), Uint128::new(100u128))],
        );

        let withdrawer_addr = Addr::unchecked("withdrawer");

        // Initialize markets
        let ma_token_1_addr = Addr::unchecked("matoken1");
        let market_1 = Market {
            ma_token_address: ma_token_1_addr.clone(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let ma_token_2_addr = Addr::unchecked("matoken2");
        let market_2 = Market {
            ma_token_address: ma_token_2_addr,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let ma_token_3_addr = Addr::unchecked("matoken3");
        let market_3 = Market {
            ma_token_address: ma_token_3_addr.clone(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let market_1_initial = th_init_market(deps.as_mut(), b"token1", &market_1);
        let market_2_initial = th_init_market(deps.as_mut(), b"token2", &market_2);
        let market_3_initial = th_init_market(deps.as_mut(), b"token3", &market_3);

        // Initialize user with market_1 and market_3 as collaterals
        // User borrows market_2
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
        set_bit(&mut user.collateral_assets, market_3_initial.index).unwrap();
        set_bit(&mut user.borrowed_assets, market_2_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &withdrawer_addr, &user)
            .unwrap();

        // Set the querier to return collateral balances (ma_token_1 and ma_token_3)
        let ma_token_1_balance_scaled = Uint128::new(100_000) * SCALING_FACTOR;
        deps.querier.set_cw20_balances(
            ma_token_1_addr,
            &[(withdrawer_addr.clone(), ma_token_1_balance_scaled.into())],
        );
        let ma_token_3_balance_scaled = Uint128::new(600_000) * SCALING_FACTOR;
        deps.querier.set_cw20_balances(
            ma_token_3_addr,
            &[(withdrawer_addr.clone(), ma_token_3_balance_scaled.into())],
        );

        // Set user to have positive debt amount in debt asset
        // Uncollateralized debt shouldn't count for health factor
        let token_2_debt_scaled = Uint128::new(200_000) * SCALING_FACTOR;
        let debt = Debt {
            amount_scaled: token_2_debt_scaled,
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(200_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS
            .save(deps.as_mut().storage, (b"token2", &withdrawer_addr), &debt)
            .unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                (b"token3", &withdrawer_addr),
                &uncollateralized_debt,
            )
            .unwrap();

        // Set the querier to return native exchange rates
        let token_1_exchange_rate = Decimal::from_ratio(3u128, 1u128);
        let token_2_exchange_rate = Decimal::from_ratio(2u128, 1u128);
        let token_3_exchange_rate = Decimal::from_ratio(1u128, 1u128);

        deps.querier
            .set_oracle_price(b"token1".to_vec(), token_1_exchange_rate);
        deps.querier
            .set_oracle_price(b"token2".to_vec(), token_2_exchange_rate);
        deps.querier
            .set_oracle_price(b"token3".to_vec(), token_3_exchange_rate);

        let env = mock_env(MockEnvParams::default());
        let info = mock_info("withdrawer");

        // Calculate how much to withdraw to have health factor equal to one
        let how_much_to_withdraw = {
            let token_1_weighted_lt_in_uusd = compute_underlying_amount(
                ma_token_1_balance_scaled,
                get_updated_liquidity_index(&market_1_initial, env.block.time.seconds()).unwrap(),
            )
            .unwrap()
                * market_1_initial.liquidation_threshold
                * token_1_exchange_rate;
            let token_3_weighted_lt_in_uusd = compute_underlying_amount(
                ma_token_3_balance_scaled,
                get_updated_liquidity_index(&market_3_initial, env.block.time.seconds()).unwrap(),
            )
            .unwrap()
                * market_3_initial.liquidation_threshold
                * token_3_exchange_rate;
            let weighted_liquidation_threshold_in_uusd =
                token_1_weighted_lt_in_uusd + token_3_weighted_lt_in_uusd;

            let total_collateralized_debt_in_uusd = compute_underlying_amount(
                token_2_debt_scaled,
                get_updated_borrow_index(&market_2_initial, env.block.time.seconds()).unwrap(),
            )
            .unwrap()
                * token_2_exchange_rate;

            // How much to withdraw in uusd to have health factor equal to one
            let how_much_to_withdraw_in_uusd = Decimal::divide_uint128_by_decimal(
                weighted_liquidation_threshold_in_uusd - total_collateralized_debt_in_uusd,
                market_3_initial.liquidation_threshold,
            )
            .unwrap();
            Decimal::divide_uint128_by_decimal(how_much_to_withdraw_in_uusd, token_3_exchange_rate)
                .unwrap()
        };

        // Withdraw token3 with failure
        // The withdraw amount needs to be a little bit greater to have health factor less than one
        {
            let withdraw_amount = how_much_to_withdraw + Uint128::from(10u128);
            let msg = ExecuteMsg::Withdraw {
                asset: Asset::Native {
                    denom: "token3".to_string(),
                },
                amount: Some(withdraw_amount),
                recipient: None,
            };
            let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InvalidHealthFactorAfterWithdraw {}
            );
        }

        // Withdraw token3 with success
        // The withdraw amount needs to be a little bit smaller to have health factor greater than one
        {
            let withdraw_amount = how_much_to_withdraw - Uint128::from(10u128);
            let msg = ExecuteMsg::Withdraw {
                asset: Asset::Native {
                    denom: "token3".to_string(),
                },
                amount: Some(withdraw_amount),
                recipient: None,
            };
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            let withdraw_amount_scaled = get_scaled_liquidity_amount(
                withdraw_amount,
                &market_3_initial,
                env.block.time.seconds(),
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: "matoken3".to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                            user: withdrawer_addr.to_string(),
                            amount: withdraw_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: withdrawer_addr.to_string(),
                        amount: vec![deduct_tax(
                            deps.as_ref(),
                            Coin {
                                denom: String::from("token3"),
                                amount: withdraw_amount,
                            }
                        )
                        .unwrap()],
                    })),
                ]
            );
        }
    }

    #[test]
    fn test_withdraw_total_balance() {
        // Withdraw native token
        let initial_available_liquidity = 12000000u128;
        let mut deps = th_setup(&[coin(initial_available_liquidity, "somecoin")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("somecoin"), Uint128::new(100u128))],
        );

        let initial_liquidity_index = Decimal::from_ratio(15u128, 10u128);
        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: initial_liquidity_index,
            borrow_index: Decimal::from_ratio(2u128, 1u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(1u128, 10u128),
            debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
            indexes_last_updated: 10000000,
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let withdrawer_balance_scaled = Uint128::new(123_456) * SCALING_FACTOR;
        let seconds_elapsed = 2000u64;

        deps.querier.set_cw20_balances(
            Addr::unchecked("matoken"),
            &[(
                Addr::unchecked("withdrawer"),
                withdrawer_balance_scaled.into(),
            )],
        );

        let market_initial = th_init_market(deps.as_mut(), b"somecoin", &mock_market);
        MARKET_REFERENCES_BY_MA_TOKEN
            .save(
                deps.as_mut().storage,
                &Addr::unchecked("matoken"),
                &(b"somecoin".to_vec()),
            )
            .unwrap();

        // Mark the market as collateral for the user
        let withdrawer_addr = Addr::unchecked("withdrawer");
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &withdrawer_addr, &user)
            .unwrap();
        // Check if user has set bit for collateral
        assert!(get_bit(user.collateral_assets, market_initial.index).unwrap());

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: None,
            recipient: None,
        };

        let env = mock_env_at_block_time(mock_market.indexes_last_updated + seconds_elapsed);
        let info = mock_info("withdrawer");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let market = MARKETS.load(&deps.storage, b"somecoin").unwrap();

        let withdrawer_balance = compute_underlying_amount(
            withdrawer_balance_scaled,
            get_updated_liquidity_index(
                &market_initial,
                market_initial.indexes_last_updated + seconds_elapsed,
            )
            .unwrap(),
        )
        .unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market_initial,
            mock_market.indexes_last_updated + seconds_elapsed,
            initial_available_liquidity,
            TestUtilizationDeltas {
                less_liquidity: withdrawer_balance.into(),
                ..Default::default()
            },
        );

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "matoken".to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                        recipient: "protocol_rewards_collector".to_string(),
                        amount: compute_scaled_amount(
                            expected_params.protocol_rewards_to_distribute,
                            expected_params.liquidity_index,
                            ScalingOperation::Liquidity
                        )
                        .unwrap(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "matoken".to_string(),
                    msg: to_binary(&ma_token::msg::ExecuteMsg::Burn {
                        user: withdrawer_addr.to_string(),
                        amount: withdrawer_balance_scaled.into(),
                    })
                    .unwrap(),
                    funds: vec![]
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: withdrawer_addr.to_string(),
                    amount: vec![deduct_tax(
                        deps.as_ref(),
                        Coin {
                            denom: String::from("somecoin"),
                            amount: withdrawer_balance.into(),
                        }
                    )
                    .unwrap()],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("asset", "somecoin"),
                attr("user", "withdrawer"),
                attr("recipient", "withdrawer"),
                attr("burn_amount", withdrawer_balance_scaled.to_string()),
                attr("withdraw_amount", withdrawer_balance.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_collateral_position_changed_event(
                    "somecoin",
                    false,
                    "withdrawer".to_string()
                ),
                th_build_interests_updated_event("somecoin", &expected_params)
            ]
        );

        assert_eq!(market.borrow_rate, expected_params.borrow_rate);
        assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);
        assert_eq!(market.liquidity_index, expected_params.liquidity_index);
        assert_eq!(market.borrow_index, expected_params.borrow_index);

        // User should have unset bit for collateral after full withdraw
        let user = USERS.load(&deps.storage, &withdrawer_addr).unwrap();
        assert!(!get_bit(user.collateral_assets, market_initial.index).unwrap());
    }

    #[test]
    fn test_withdraw_without_existing_position() {
        // Withdraw native token
        let initial_available_liquidity = 12000000u128;
        let mut deps = th_setup(&[coin(initial_available_liquidity, "somecoin")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("somecoin"), Uint128::new(100u128))],
        );

        deps.querier.set_cw20_balances(
            Addr::unchecked("matoken"),
            &[
                (
                    Addr::unchecked("withdrawer"),
                    Uint128::new(2_000_000) * SCALING_FACTOR,
                ),
                (
                    Addr::unchecked("protocol_rewards_collector"),
                    Uint128::new(2_000_000) * SCALING_FACTOR,
                ),
            ],
        );

        let market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        th_init_market(deps.as_mut(), b"somecoin", &market);

        let msg = ExecuteMsg::Withdraw {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: None,
            recipient: None,
        };

        // normal address cannot withdraw without an existing position
        {
            let info = mock_info("withdrawer");
            let env = mock_env(MockEnvParams::default());
            let error = execute(deps.as_mut(), env, info, msg.clone()).unwrap_err();
            assert_eq!(error, ContractError::ExistingUserPositionRequired {});
        }

        // protocol_rewards_collector can withdraw without an existing position
        {
            let info = mock_info("protocol_rewards_collector");
            let env = mock_env(MockEnvParams::default());
            execute(deps.as_mut(), env, info, msg).unwrap();
        }
    }

    #[test]
    fn test_borrow_and_repay() {
        // NOTE: available liquidity stays fixed as the test environment does not get changes in
        // contract balances on subsequent calls. They would change from call to call in practice
        let available_liquidity_cw20 = 1000000000u128; // cw20
        let available_liquidity_native = 2000000000u128; // native
        let mut deps = th_setup(&[coin(available_liquidity_native, "borrowedcoinnative")]);

        let cw20_contract_addr = Addr::unchecked("borrowedcoincw20");
        deps.querier.set_cw20_balances(
            cw20_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(available_liquidity_cw20),
            )],
        );

        deps.querier
            .set_oracle_price(b"borrowedcoinnative".to_vec(), Decimal::one());
        deps.querier
            .set_oracle_price(b"depositedcoin".to_vec(), Decimal::one());
        deps.querier
            .set_oracle_price(b"borrowedcoincw20".to_vec(), Decimal::one());

        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("borrowedcoinnative"), Uint128::new(100u128))],
        );

        let mock_market_1 = Market {
            ma_token_address: Addr::unchecked("matoken1"),
            borrow_index: Decimal::from_ratio(12u128, 10u128),
            liquidity_index: Decimal::from_ratio(8u128, 10u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(1u128, 100u128),
            debt_total_scaled: Uint128::zero(),
            indexes_last_updated: 10000000,
            asset_type: AssetType::Cw20,
            ..Default::default()
        };
        let mock_market_2 = Market {
            ma_token_address: Addr::unchecked("matoken2"),
            borrow_index: Decimal::one(),
            liquidity_index: Decimal::one(),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let mock_market_3 = Market {
            ma_token_address: Addr::unchecked("matoken3"),
            borrow_index: Decimal::one(),
            liquidity_index: Decimal::from_ratio(11u128, 10u128),
            max_loan_to_value: Decimal::from_ratio(7u128, 10u128),
            borrow_rate: Decimal::from_ratio(30u128, 100u128),
            reserve_factor: Decimal::from_ratio(3u128, 100u128),
            liquidity_rate: Decimal::from_ratio(20u128, 100u128),
            debt_total_scaled: Uint128::zero(),
            indexes_last_updated: 10000000,
            asset_type: AssetType::Native,
            ..Default::default()
        };

        // should get index 0
        let market_1_initial =
            th_init_market(deps.as_mut(), cw20_contract_addr.as_bytes(), &mock_market_1);
        // should get index 1
        let market_2_initial = th_init_market(deps.as_mut(), b"borrowedcoinnative", &mock_market_2);
        // should get index 2
        let market_collateral = th_init_market(deps.as_mut(), b"depositedcoin", &mock_market_3);

        let borrower_addr = Addr::unchecked("borrower");

        // Set user as having the market_collateral deposited
        let mut user = User::default();

        set_bit(&mut user.collateral_assets, market_collateral.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        // Set the querier to return a certain collateral balance
        let deposit_coin_address = Addr::unchecked("matoken3");
        deps.querier.set_cw20_balances(
            deposit_coin_address,
            &[(borrower_addr.clone(), Uint128::new(10000) * SCALING_FACTOR)],
        );

        // *
        // Borrow cw20 token
        // *
        let block_time = mock_market_1.indexes_last_updated + 10000u64;
        let borrow_amount = 2400u128;

        let msg = ExecuteMsg::Borrow {
            asset: Asset::Cw20 {
                contract_addr: cw20_contract_addr.to_string(),
            },
            amount: Uint128::from(borrow_amount),
            recipient: None,
        };

        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_params_cw20 = th_get_expected_indices_and_rates(
            &market_1_initial,
            block_time,
            available_liquidity_cw20,
            TestUtilizationDeltas {
                less_liquidity: borrow_amount,
                more_debt: borrow_amount,
                ..Default::default()
            },
        );

        // check correct messages and logging
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: borrower_addr.to_string(),
                    amount: borrow_amount.into(),
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "borrow"),
                attr("asset", "borrowedcoincw20"),
                attr("user", "borrower"),
                attr("recipient", "borrower"),
                attr("amount", borrow_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_debt_position_changed_event("borrowedcoincw20", true, "borrower".to_string()),
                th_build_interests_updated_event("borrowedcoincw20", &expected_params_cw20)
            ]
        );

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());
        assert!(!get_bit(user.borrowed_assets, 1).unwrap());

        let debt = DEBTS
            .load(
                &deps.storage,
                (cw20_contract_addr.as_bytes(), &borrower_addr),
            )
            .unwrap();
        let expected_debt_scaled_1_after_borrow = compute_scaled_amount(
            Uint128::from(borrow_amount),
            expected_params_cw20.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();

        let market_1_after_borrow = MARKETS
            .load(&deps.storage, cw20_contract_addr.as_bytes())
            .unwrap();

        assert_eq!(expected_debt_scaled_1_after_borrow, debt.amount_scaled);
        assert_eq!(
            expected_debt_scaled_1_after_borrow,
            market_1_after_borrow.debt_total_scaled
        );
        assert_eq!(
            expected_params_cw20.borrow_rate,
            market_1_after_borrow.borrow_rate
        );
        assert_eq!(
            expected_params_cw20.liquidity_rate,
            market_1_after_borrow.liquidity_rate
        );

        // *
        // Borrow cw20 token (again)
        // *
        let borrow_amount = 1200u128;
        let block_time = market_1_after_borrow.indexes_last_updated + 20000u64;

        let msg = ExecuteMsg::Borrow {
            asset: Asset::Cw20 {
                contract_addr: cw20_contract_addr.to_string(),
            },
            amount: Uint128::from(borrow_amount),
            recipient: None,
        };

        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");

        execute(deps.as_mut(), env, info, msg).unwrap();

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());
        assert!(!get_bit(user.borrowed_assets, 1).unwrap());

        let expected_params_cw20 = th_get_expected_indices_and_rates(
            &market_1_after_borrow,
            block_time,
            available_liquidity_cw20,
            TestUtilizationDeltas {
                less_liquidity: borrow_amount,
                more_debt: borrow_amount,
                ..Default::default()
            },
        );
        let debt = DEBTS
            .load(
                &deps.storage,
                (cw20_contract_addr.as_bytes(), &borrower_addr),
            )
            .unwrap();
        let market_1_after_borrow_again = MARKETS
            .load(&deps.storage, cw20_contract_addr.as_bytes())
            .unwrap();

        let expected_debt_scaled_1_after_borrow_again = expected_debt_scaled_1_after_borrow
            + compute_scaled_amount(
                Uint128::from(borrow_amount),
                expected_params_cw20.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();
        assert_eq!(
            expected_debt_scaled_1_after_borrow_again,
            debt.amount_scaled
        );
        assert_eq!(
            expected_debt_scaled_1_after_borrow_again,
            market_1_after_borrow_again.debt_total_scaled
        );
        assert_eq!(
            expected_params_cw20.borrow_rate,
            market_1_after_borrow_again.borrow_rate
        );
        assert_eq!(
            expected_params_cw20.liquidity_rate,
            market_1_after_borrow_again.liquidity_rate
        );

        // *
        // Borrow native coin
        // *

        let borrow_amount = 4000u128;
        let block_time = market_1_after_borrow_again.indexes_last_updated + 3000u64;
        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: String::from("borrowedcoinnative"),
            },
            amount: Uint128::from(borrow_amount),
            recipient: None,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());
        assert!(get_bit(user.borrowed_assets, 1).unwrap());

        let expected_params_native = th_get_expected_indices_and_rates(
            &market_2_initial,
            block_time,
            available_liquidity_native,
            TestUtilizationDeltas {
                less_liquidity: borrow_amount,
                more_debt: borrow_amount,
                ..Default::default()
            },
        );

        // check correct messages and logging
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "borrower".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: String::from("borrowedcoinnative"),
                        amount: borrow_amount.into(),
                    }
                )
                .unwrap()],
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "borrow"),
                attr("asset", "borrowedcoinnative"),
                attr("user", "borrower"),
                attr("recipient", "borrower"),
                attr("amount", borrow_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_debt_position_changed_event(
                    "borrowedcoinnative",
                    true,
                    "borrower".to_string()
                ),
                th_build_interests_updated_event("borrowedcoinnative", &expected_params_native)
            ]
        );

        let debt2 = DEBTS
            .load(&deps.storage, (b"borrowedcoinnative", &borrower_addr))
            .unwrap();
        let market_2_after_borrow_2 = MARKETS.load(&deps.storage, b"borrowedcoinnative").unwrap();

        let expected_debt_scaled_2_after_borrow_2 = compute_scaled_amount(
            Uint128::from(borrow_amount),
            expected_params_native.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();
        assert_eq!(expected_debt_scaled_2_after_borrow_2, debt2.amount_scaled);
        assert_eq!(
            expected_debt_scaled_2_after_borrow_2,
            market_2_after_borrow_2.debt_total_scaled
        );
        assert_eq!(
            expected_params_native.borrow_rate,
            market_2_after_borrow_2.borrow_rate
        );
        assert_eq!(
            expected_params_native.liquidity_rate,
            market_2_after_borrow_2.liquidity_rate
        );

        // *
        // Borrow native coin again (should fail due to insufficient collateral)
        // *

        let env = mock_env(MockEnvParams::default());
        let info = mock_info("borrower");
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: String::from("borrowedcoinnative"),
            },
            amount: Uint128::from(83968_u128),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::BorrowAmountExceedsGivenCollateral {}
        );

        // *
        // Repay zero native debt (should fail)
        // *
        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        let msg = ExecuteMsg::RepayNative {
            denom: String::from("borrowedcoinnative"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InvalidNativeCoinsSent {
                denom: "borrowedcoinnative".to_string()
            }
        );

        // *
        // Repay some native debt
        // *
        let repay_amount = 2000u128;
        let block_time = market_2_after_borrow_2.indexes_last_updated + 8000u64;
        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            "borrower",
            &[coin(repay_amount, "borrowedcoinnative")],
        );
        let msg = ExecuteMsg::RepayNative {
            denom: String::from("borrowedcoinnative"),
            on_behalf_of: None,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_params_native = th_get_expected_indices_and_rates(
            &market_2_after_borrow_2,
            block_time,
            available_liquidity_native,
            TestUtilizationDeltas {
                less_debt: repay_amount,
                ..Default::default()
            },
        );

        assert_eq!(res.messages, vec![]);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "repay"),
                attr("asset", "borrowedcoinnative"),
                attr("sender", "borrower"),
                attr("user", "borrower"),
                attr("amount", repay_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![th_build_interests_updated_event(
                "borrowedcoinnative",
                &expected_params_native
            )]
        );

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());
        assert!(get_bit(user.borrowed_assets, 1).unwrap());

        let debt2 = DEBTS
            .load(&deps.storage, (b"borrowedcoinnative", &borrower_addr))
            .unwrap();
        let market_2_after_repay_some_2 =
            MARKETS.load(&deps.storage, b"borrowedcoinnative").unwrap();

        let expected_debt_scaled_2_after_repay_some_2 = expected_debt_scaled_2_after_borrow_2
            - compute_scaled_amount(
                Uint128::from(repay_amount),
                expected_params_native.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();
        assert_eq!(
            expected_debt_scaled_2_after_repay_some_2,
            debt2.amount_scaled
        );
        assert_eq!(
            expected_debt_scaled_2_after_repay_some_2,
            market_2_after_repay_some_2.debt_total_scaled
        );
        assert_eq!(
            expected_params_native.borrow_rate,
            market_2_after_repay_some_2.borrow_rate
        );
        assert_eq!(
            expected_params_native.liquidity_rate,
            market_2_after_repay_some_2.liquidity_rate
        );

        // *
        // Repay all native debt
        // *
        let block_time = market_2_after_repay_some_2.indexes_last_updated + 10000u64;
        // need this to compute the repay amount
        let expected_params_native = th_get_expected_indices_and_rates(
            &market_2_after_repay_some_2,
            block_time,
            available_liquidity_native,
            TestUtilizationDeltas {
                less_debt: 9999999999999, // hack: Just do a big number to repay all debt,
                ..Default::default()
            },
        );

        let repay_amount: u128 = compute_underlying_amount(
            expected_debt_scaled_2_after_repay_some_2,
            expected_params_native.borrow_index,
        )
        .unwrap()
        .into();

        let env = mock_env_at_block_time(block_time);
        let info = cosmwasm_std::testing::mock_info(
            "borrower",
            &[coin(repay_amount, "borrowedcoinnative")],
        );
        let msg = ExecuteMsg::RepayNative {
            denom: String::from("borrowedcoinnative"),
            on_behalf_of: None,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(res.messages, vec![]);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "repay"),
                attr("asset", "borrowedcoinnative"),
                attr("sender", "borrower"),
                attr("user", "borrower"),
                attr("amount", repay_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                th_build_interests_updated_event("borrowedcoinnative", &expected_params_native),
                build_debt_position_changed_event(
                    "borrowedcoinnative",
                    false,
                    "borrower".to_string()
                ),
            ]
        );

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());
        assert!(!get_bit(user.borrowed_assets, 1).unwrap());

        let debt2 = DEBTS
            .load(&deps.storage, (b"borrowedcoinnative", &borrower_addr))
            .unwrap();
        let market_2_after_repay_all_2 =
            MARKETS.load(&deps.storage, b"borrowedcoinnative").unwrap();

        assert_eq!(Uint128::zero(), debt2.amount_scaled);
        assert_eq!(
            Uint128::zero(),
            market_2_after_repay_all_2.debt_total_scaled
        );

        // *
        // Repay more native debt (should fail)
        // *
        let env = mock_env(MockEnvParams::default());
        let info =
            cosmwasm_std::testing::mock_info("borrower", &[coin(2000, "borrowedcoinnative")]);
        let msg = ExecuteMsg::RepayNative {
            denom: String::from("borrowedcoinnative"),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotRepayZeroDebt {});

        // *
        // Repay all cw20 debt (and then some)
        // *
        let block_time = market_2_after_repay_all_2.indexes_last_updated + 5000u64;
        let repay_amount = 4800u128;

        let expected_params_cw20 = th_get_expected_indices_and_rates(
            &market_1_after_borrow_again,
            block_time,
            available_liquidity_cw20,
            TestUtilizationDeltas {
                less_debt: repay_amount,
                ..Default::default()
            },
        );

        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrowedcoincw20");

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            msg: to_binary(&ReceiveMsg::RepayCw20 { on_behalf_of: None }).unwrap(),
            sender: borrower_addr.to_string(),
            amount: Uint128::new(repay_amount),
        });

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_repay_amount_scaled = compute_scaled_amount(
            Uint128::from(repay_amount),
            expected_params_cw20.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();
        let expected_refund_amount: u128 = (compute_underlying_amount(
            expected_repay_amount_scaled - expected_debt_scaled_1_after_borrow_again,
            expected_params_cw20.borrow_index,
        )
        .unwrap())
        .into();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: borrower_addr.to_string(),
                    amount: expected_refund_amount.into(),
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "repay"),
                attr("asset", "borrowedcoincw20"),
                attr("sender", "borrower"),
                attr("user", "borrower"),
                attr(
                    "amount",
                    Uint128::new(repay_amount - expected_refund_amount).to_string()
                ),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                th_build_interests_updated_event("borrowedcoincw20", &expected_params_cw20),
                build_debt_position_changed_event(
                    "borrowedcoincw20",
                    false,
                    "borrower".to_string()
                ),
            ]
        );
        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(!get_bit(user.borrowed_assets, 0).unwrap());
        assert!(!get_bit(user.borrowed_assets, 1).unwrap());

        let debt1 = DEBTS
            .load(
                &deps.storage,
                (cw20_contract_addr.as_bytes(), &borrower_addr),
            )
            .unwrap();
        let market_1_after_repay_1 = MARKETS
            .load(&deps.storage, cw20_contract_addr.as_bytes())
            .unwrap();
        assert_eq!(Uint128::zero(), debt1.amount_scaled);
        assert_eq!(Uint128::zero(), market_1_after_repay_1.debt_total_scaled);
    }

    #[test]
    fn test_cannot_repay_if_market_inactive() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: false,
            deposit_enabled: true,
            borrow_enabled: true,
            ..Default::default()
        };
        let _market = th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("borrower", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::RepayNative {
            denom: "somecoin".to_string(),
            on_behalf_of: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::MarketNotActive {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_repay_on_behalf_of() {
        let available_liquidity_native = 1000000000u128;
        let mut deps = th_setup(&[coin(available_liquidity_native, "borrowedcoinnative")]);

        deps.querier
            .set_oracle_price(b"depositedcoinnative".to_vec(), Decimal::one());
        deps.querier
            .set_oracle_price(b"borrowedcoinnative".to_vec(), Decimal::one());

        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("borrowedcoinnative"), Uint128::new(100u128))],
        );

        let mock_market_1 = Market {
            ma_token_address: Addr::unchecked("matoken1"),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let mock_market_2 = Market {
            ma_token_address: Addr::unchecked("matoken2"),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };

        let market_1_initial =
            th_init_market(deps.as_mut(), b"depositedcoinnative", &mock_market_1); // collateral
        let market_2_initial = th_init_market(deps.as_mut(), b"borrowedcoinnative", &mock_market_2);

        let borrower_addr = Addr::unchecked("borrower");
        let user_addr = Addr::unchecked("user");

        // Set user as having the market_1_initial (collateral) deposited
        let mut user = User::default();

        set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        // Set the querier to return a certain collateral balance
        let deposit_coin_address = Addr::unchecked("matoken1");
        deps.querier.set_cw20_balances(
            deposit_coin_address,
            &[(borrower_addr.clone(), Uint128::new(10000) * SCALING_FACTOR)],
        );

        // *
        // 'borrower' borrows native coin
        // *
        let borrow_amount = 4000u128;
        let env = mock_env(MockEnvParams::default());
        let info = mock_info(borrower_addr.as_str());
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: String::from("borrowedcoinnative"),
            },
            amount: Uint128::from(borrow_amount),
            recipient: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, market_2_initial.index).unwrap());

        // *
        // 'user' repays debt on behalf of 'borrower'
        // *
        let repay_amount = borrow_amount;
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info(
            user_addr.as_str(),
            &[coin(repay_amount, "borrowedcoinnative")],
        );
        let msg = ExecuteMsg::RepayNative {
            denom: String::from("borrowedcoinnative"),
            on_behalf_of: Some(borrower_addr.to_string()),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // 'user' should not be saved
        let _user = USERS.load(&deps.storage, &user_addr).unwrap_err();

        // Debt for 'user' should not exist
        let debt = DEBTS
            .may_load(&deps.storage, (b"borrowedcoinnative", &user_addr))
            .unwrap();
        assert!(debt.is_none());

        // Debt for 'borrower' should be repayed
        let debt = DEBTS
            .load(&deps.storage, (b"borrowedcoinnative", &borrower_addr))
            .unwrap();
        assert_eq!(debt.amount_scaled, Uint128::zero());

        // 'borrower' should have unset bit for debt after full repay
        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(!get_bit(user.borrowed_assets, market_2_initial.index).unwrap());

        // Check msgs and attributes
        assert_eq!(res.messages, vec![]);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "repay"),
                attr("asset", "borrowedcoinnative"),
                attr("sender", "user"),
                attr("user", "borrower"),
                attr("amount", repay_amount.to_string()),
            ]
        );
    }

    #[test]
    fn test_repay_uncollateralized_loan_on_behalf_of() {
        let mut deps = th_setup(&[]);

        let repayer_addr = Addr::unchecked("repayer");
        let another_user_addr = Addr::unchecked("another_user");

        UNCOLLATERALIZED_LOAN_LIMITS
            .save(
                deps.as_mut().storage,
                (b"somecoin", &another_user_addr),
                &Uint128::new(1000u128),
            )
            .unwrap();

        let env = mock_env(MockEnvParams::default());
        let info =
            cosmwasm_std::testing::mock_info(repayer_addr.as_str(), &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::RepayNative {
            denom: "somecoin".to_string(),
            on_behalf_of: Some(another_user_addr.to_string()),
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::CannotRepayUncollateralizedLoanOnBehalfOf {}
        );
    }

    #[test]
    fn test_borrow_uusd() {
        let initial_liquidity = 10000000;
        let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
        let block_time = 1;

        let borrower_addr = Addr::unchecked("borrower");
        let ltv = Decimal::from_ratio(7u128, 10u128);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::one(),
            max_loan_to_value: ltv,
            borrow_index: Decimal::from_ratio(20u128, 10u128),
            borrow_rate: Decimal::one(),
            liquidity_rate: Decimal::one(),
            debt_total_scaled: Uint128::zero(),
            indexes_last_updated: block_time,
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"uusd", &mock_market);

        // Set tax data for uusd
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("uusd"), Uint128::new(100u128))],
        );

        // Set user as having the market_collateral deposited
        let deposit_amount_scaled = Uint128::new(110_000) * SCALING_FACTOR;
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        // Set the querier to return collateral balance
        let deposit_coin_address = Addr::unchecked("matoken");
        deps.querier.set_cw20_balances(
            deposit_coin_address,
            &[(borrower_addr.clone(), deposit_amount_scaled.into())],
        );

        // borrow with insufficient collateral, should fail
        let new_block_time = 120u64;
        let time_elapsed = new_block_time - market.indexes_last_updated;
        let liquidity_index = calculate_applied_linear_interest_rate(
            market.liquidity_index,
            market.liquidity_rate,
            time_elapsed,
        )
        .unwrap();
        let collateral = compute_underlying_amount(deposit_amount_scaled, liquidity_index).unwrap();
        let max_to_borrow = collateral * ltv;
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "uusd".to_string(),
            },
            amount: max_to_borrow + Uint128::from(1u128),
            recipient: None,
        };
        let env = mock_env_at_block_time(new_block_time);
        let info = mock_info("borrower");
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::BorrowAmountExceedsGivenCollateral {}
        );

        let valid_amount = max_to_borrow - Uint128::from(1000u128);
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "uusd".to_string(),
            },
            amount: valid_amount,
            recipient: None,
        };
        let env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        execute(deps.as_mut(), env, info, msg).unwrap();

        let market_after_borrow = MARKETS.load(&deps.storage, b"uusd").unwrap();

        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());

        let debt = DEBTS
            .load(&deps.storage, (b"uusd", &borrower_addr))
            .unwrap();

        assert_eq!(
            valid_amount,
            compute_underlying_amount(debt.amount_scaled, market_after_borrow.borrow_index)
                .unwrap()
        );
    }

    #[test]
    fn test_borrow_full_liquidity_and_then_repay() {
        let initial_liquidity = 50000;
        let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);
        let info = mock_info("borrower");
        let borrower_addr = Addr::unchecked("borrower");
        let block_time = 1;
        let ltv = Decimal::one();

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::one(),
            max_loan_to_value: ltv,
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            liquidity_rate: Decimal::one(),
            debt_total_scaled: Uint128::zero(),
            reserve_factor: Decimal::from_ratio(12u128, 100u128),
            indexes_last_updated: block_time,
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"uusd", &mock_market);

        // Set tax data for uusd
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("uusd"), Uint128::new(100u128))],
        );

        // User should have amount of collateral more than initial liquidity in order to borrow full liquidity
        let deposit_amount = initial_liquidity + 1000u128;
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        // Set the querier to return collateral balance
        let deposit_coin_address = Addr::unchecked("matoken");
        deps.querier.set_cw20_balances(
            deposit_coin_address,
            &[(
                borrower_addr.clone(),
                Uint128::new(deposit_amount) * SCALING_FACTOR,
            )],
        );

        // Borrow full liquidity
        {
            let env = mock_env_at_block_time(block_time);
            let msg = ExecuteMsg::Borrow {
                asset: Asset::Native {
                    denom: "uusd".to_string(),
                },
                amount: initial_liquidity.into(),
                recipient: None,
            };
            let _res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

            let market_after_borrow = MARKETS.load(&deps.storage, b"uusd").unwrap();
            let debt_total = compute_underlying_amount(
                market_after_borrow.debt_total_scaled,
                market_after_borrow.borrow_index,
            )
            .unwrap();
            assert_eq!(debt_total, initial_liquidity.into());
        }

        let new_block_time = 12000u64;
        // We need to update balance after borrowing
        deps.querier.set_contract_balances(&[coin(0, "uusd")]);

        // Try to borrow more than available liquidity
        {
            let env = mock_env_at_block_time(new_block_time);
            let msg = ExecuteMsg::Borrow {
                asset: Asset::Native {
                    denom: "uusd".to_string(),
                },
                amount: 100u128.into(),
                recipient: None,
            };
            let error_res = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::OperationExceedsAvailableLiquidity {}
            );
        }

        // Repay part of the debt
        {
            let env = mock_env_at_block_time(new_block_time);
            let info = cosmwasm_std::testing::mock_info("borrower", &[coin(2000, "uusd")]);
            let msg = ExecuteMsg::RepayNative {
                denom: String::from("uusd"),
                on_behalf_of: None,
            };
            // check that repay succeeds
            execute(deps.as_mut(), env, info, msg).unwrap();
        }
    }

    #[test]
    fn test_borrow_collateral_check() {
        // NOTE: available liquidity stays fixed as the test environment does not get changes in
        // contract balances on subsequent calls. They would change from call to call in practice
        let available_liquidity_1 = 1000000000u128;
        let available_liquidity_2 = 2000000000u128;
        let available_liquidity_3 = 3000000000u128;
        let mut deps = th_setup(&[
            coin(available_liquidity_2, "depositedcoin2"),
            coin(available_liquidity_3, "uusd"),
        ]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("depositedcoin2"), Uint128::new(100u128))],
        );

        let cw20_contract_addr = Addr::unchecked("depositedcoin1");
        deps.querier.set_cw20_balances(
            cw20_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(available_liquidity_1),
            )],
        );

        let exchange_rate_1 = Decimal::one();
        let exchange_rate_2 = Decimal::from_ratio(15u128, 4u128);
        let exchange_rate_3 = Decimal::one();

        deps.querier
            .set_oracle_price(cw20_contract_addr.as_bytes().to_vec(), exchange_rate_1);
        deps.querier
            .set_oracle_price(b"depositedcoin2".to_vec(), exchange_rate_2);
        // NOTE: uusd price (asset3) should be set to 1 by the oracle helper

        let exchange_rates = &[(String::from("depositedcoin2"), exchange_rate_2)];
        deps.querier
            .set_native_exchange_rates(String::from("uusd"), &exchange_rates[..]);

        let mock_market_1 = Market {
            ma_token_address: Addr::unchecked("matoken1"),
            max_loan_to_value: Decimal::from_ratio(8u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::from_ratio(1u128, 2u128),
            asset_type: AssetType::Cw20,
            ..Default::default()
        };
        let mock_market_2 = Market {
            ma_token_address: Addr::unchecked("matoken2"),
            max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::from_ratio(1u128, 2u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let mock_market_3 = Market {
            ma_token_address: Addr::unchecked("matoken3"),
            max_loan_to_value: Decimal::from_ratio(4u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::from_ratio(1u128, 2u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };

        // should get index 0
        let market_1_initial =
            th_init_market(deps.as_mut(), cw20_contract_addr.as_bytes(), &mock_market_1);
        // should get index 1
        let market_2_initial = th_init_market(deps.as_mut(), b"depositedcoin2", &mock_market_2);
        // should get index 2
        let market_3_initial = th_init_market(deps.as_mut(), b"uusd", &mock_market_3);

        let borrower_addr = Addr::unchecked("borrower");

        // Set user as having all the markets as collateral
        let mut user = User::default();

        set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
        set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
        set_bit(&mut user.collateral_assets, market_3_initial.index).unwrap();

        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        let ma_token_address_1 = Addr::unchecked("matoken1");
        let ma_token_address_2 = Addr::unchecked("matoken2");
        let ma_token_address_3 = Addr::unchecked("matoken3");

        let balance_1 = Uint128::new(4_000_000) * SCALING_FACTOR;
        let balance_2 = Uint128::new(7_000_000) * SCALING_FACTOR;
        let balance_3 = Uint128::new(3_000_000) * SCALING_FACTOR;

        // Set the querier to return a certain collateral balance
        deps.querier.set_cw20_balances(
            ma_token_address_1,
            &[(borrower_addr.clone(), balance_1.into())],
        );
        deps.querier.set_cw20_balances(
            ma_token_address_2,
            &[(borrower_addr.clone(), balance_2.into())],
        );
        deps.querier
            .set_cw20_balances(ma_token_address_3, &[(borrower_addr, balance_3.into())]);

        let max_borrow_allowed_in_uusd = (market_1_initial.max_loan_to_value
            * compute_underlying_amount(balance_1, market_1_initial.liquidity_index).unwrap()
            * exchange_rate_1)
            + (market_2_initial.max_loan_to_value
                * compute_underlying_amount(balance_2, market_2_initial.liquidity_index).unwrap()
                * exchange_rate_2)
            + (market_3_initial.max_loan_to_value
                * compute_underlying_amount(balance_3, market_3_initial.liquidity_index).unwrap()
                * exchange_rate_3);
        let exceeding_borrow_amount =
            Decimal::divide_uint128_by_decimal(max_borrow_allowed_in_uusd, exchange_rate_2)
                .unwrap()
                + Uint128::from(100_u64);
        let permissible_borrow_amount =
            Decimal::divide_uint128_by_decimal(max_borrow_allowed_in_uusd, exchange_rate_2)
                .unwrap()
                - Uint128::from(100_u64);

        // borrow above the allowed amount given current collateral, should fail
        let borrow_msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "depositedcoin2".to_string(),
            },
            amount: exceeding_borrow_amount,
            recipient: None,
        };
        let env = mock_env(MockEnvParams::default());
        let info = mock_info("borrower");
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), borrow_msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::BorrowAmountExceedsGivenCollateral {}
        );

        // borrow permissible amount given current collateral, should succeed
        let borrow_msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "depositedcoin2".to_string(),
            },
            amount: permissible_borrow_amount,
            recipient: None,
        };
        execute(deps.as_mut(), env, info, borrow_msg).unwrap();
    }

    #[test]
    fn test_cannot_borrow_if_market_not_active() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: false,
            borrow_enabled: true,
            ..Default::default()
        };
        th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        // Check error when borrowing not allowed on market
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("borrower", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: Uint128::new(1000),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::MarketNotActive {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_cannot_borrow_if_market_not_enabled() {
        let mut deps = th_setup(&[]);

        let mock_market = Market {
            ma_token_address: Addr::unchecked("ma_somecoin"),
            asset_type: AssetType::Native,
            active: true,
            borrow_enabled: false,
            ..Default::default()
        };
        th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        // Check error when borrowing not allowed on market
        let env = mock_env(MockEnvParams::default());
        let info = cosmwasm_std::testing::mock_info("borrower", &[coin(110000, "somecoin")]);
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: Uint128::new(1000),
            recipient: None,
        };
        let error_res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::BorrowNotEnabled {
                asset: "somecoin".to_string()
            }
        );
    }

    #[test]
    fn test_borrow_and_send_funds_to_another_user() {
        let initial_liquidity = 10000000;
        let mut deps = th_setup(&[coin(initial_liquidity, "uusd")]);

        let borrower_addr = Addr::unchecked("borrower");
        let another_user_addr = Addr::unchecked("another_user");

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(5u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"uusd", &mock_market);

        // Set tax data for uusd
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("uusd"), Uint128::new(100u128))],
        );

        // Set user as having the market_collateral deposited
        let deposit_amount_scaled = Uint128::new(100_000) * SCALING_FACTOR;
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &borrower_addr, &user)
            .unwrap();

        // Set the querier to return collateral balance
        let deposit_coin_address = Addr::unchecked("matoken");
        deps.querier.set_cw20_balances(
            deposit_coin_address,
            &[(borrower_addr.clone(), deposit_amount_scaled.into())],
        );

        let borrow_amount = Uint128::from(1000u128);
        let msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "uusd".to_string(),
            },
            amount: borrow_amount,
            recipient: Some(another_user_addr.to_string()),
        };
        let env = mock_env(MockEnvParams::default());
        let info = mock_info("borrower");
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let market_after_borrow = MARKETS.load(&deps.storage, b"uusd").unwrap();

        // 'borrower' has bit set for the borrowed asset of the market
        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, market.index).unwrap());

        // Debt for 'borrower' should exist
        let debt = DEBTS
            .load(&deps.storage, (b"uusd", &borrower_addr))
            .unwrap();
        assert_eq!(
            borrow_amount,
            compute_underlying_amount(debt.amount_scaled, market_after_borrow.borrow_index)
                .unwrap()
        );

        // Debt for 'another_user' should not exist
        let debt = DEBTS
            .may_load(&deps.storage, (b"uusd", &another_user_addr))
            .unwrap();
        assert!(debt.is_none());

        // Check msgs and attributes (funds should be sent to 'another_user')
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: another_user_addr.to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: String::from("uusd"),
                        amount: borrow_amount,
                    }
                )
                .unwrap()],
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "borrow"),
                attr("asset", "uusd"),
                attr("user", borrower_addr),
                attr("recipient", another_user_addr),
                attr("amount", borrow_amount.to_string()),
            ]
        );
    }

    #[test]
    pub fn test_liquidate() {
        // Setup
        let available_liquidity_collateral = 1_000_000_000u128;
        let available_liquidity_cw20_debt = 2_000_000_000u128;
        let available_liquidity_native_debt = 2_000_000_000u128;
        let mut deps = th_setup(&[
            coin(available_liquidity_collateral, "collateral"),
            coin(available_liquidity_native_debt, "native_debt"),
        ]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[
                (String::from("collateral"), Uint128::new(100u128)),
                (String::from("native_debt"), Uint128::new(120u128)),
            ],
        );

        let cw20_debt_contract_addr = Addr::unchecked("cw20_debt");
        let user_address = Addr::unchecked("user");
        let liquidator_address = Addr::unchecked("liquidator");

        let collateral_max_ltv = Decimal::from_ratio(5u128, 10u128);
        let collateral_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
        let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
        let collateral_price = Decimal::from_ratio(2_u128, 1_u128);
        let cw20_debt_price = Decimal::from_ratio(11_u128, 10_u128);
        let native_debt_price = Decimal::from_ratio(15_u128, 10_u128);
        let user_collateral_balance = 2_000_000;
        let user_debt = Uint128::from(3_000_000_u64); // ltv = 0.75
        let close_factor = Decimal::from_ratio(1u128, 2u128);

        let first_debt_to_repay = Uint128::from(400_000_u64);
        let first_block_time = 15_000_000;

        let second_debt_to_repay = Uint128::from(10_000_000_u64);
        let second_block_time = 16_000_000;

        // Global debt for the debt market
        let mut expected_global_cw20_debt_scaled = Uint128::new(1_800_000_000) * SCALING_FACTOR;
        let mut expected_global_native_debt_scaled = Uint128::new(500_000_000) * SCALING_FACTOR;

        CONFIG
            .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
                config.close_factor = close_factor;
                Ok(config)
            })
            .unwrap();

        deps.querier.set_cw20_balances(
            cw20_debt_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(available_liquidity_cw20_debt),
            )],
        );

        // initialize collateral and debt markets

        deps.querier
            .set_oracle_price(b"collateral".to_vec(), collateral_price);
        deps.querier
            .set_oracle_price(cw20_debt_contract_addr.as_bytes().to_vec(), cw20_debt_price);
        deps.querier
            .set_oracle_price(b"native_debt".to_vec(), native_debt_price);

        let collateral_market_ma_token_addr = Addr::unchecked("ma_collateral");
        let collateral_market = Market {
            ma_token_address: collateral_market_ma_token_addr.clone(),
            max_loan_to_value: collateral_max_ltv,
            liquidation_threshold: collateral_liquidation_threshold,
            liquidation_bonus: collateral_liquidation_bonus,
            debt_total_scaled: Uint128::new(800_000_000) * SCALING_FACTOR,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::from_ratio(2u128, 10u128),
            liquidity_rate: Decimal::from_ratio(2u128, 10u128),
            reserve_factor: Decimal::from_ratio(2u128, 100u128),
            asset_type: AssetType::Native,
            indexes_last_updated: 0,
            ..Default::default()
        };

        let cw20_debt_market = Market {
            max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
            debt_total_scaled: expected_global_cw20_debt_scaled,
            liquidity_index: Decimal::from_ratio(12u128, 10u128),
            borrow_index: Decimal::from_ratio(14u128, 10u128),
            borrow_rate: Decimal::from_ratio(2u128, 10u128),
            liquidity_rate: Decimal::from_ratio(2u128, 10u128),
            reserve_factor: Decimal::from_ratio(3u128, 100u128),
            asset_type: AssetType::Cw20,
            indexes_last_updated: 0,
            ma_token_address: Addr::unchecked("ma_cw20_debt"),
            ..Default::default()
        };

        let native_debt_market = Market {
            max_loan_to_value: Decimal::from_ratio(4u128, 10u128),
            debt_total_scaled: expected_global_native_debt_scaled,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::from_ratio(3u128, 10u128),
            liquidity_rate: Decimal::from_ratio(3u128, 10u128),
            reserve_factor: Decimal::from_ratio(2u128, 100u128),
            asset_type: AssetType::Native,
            indexes_last_updated: 0,
            ma_token_address: Addr::unchecked("ma_native_debt"),
            ..Default::default()
        };

        let collateral_market_initial =
            th_init_market(deps.as_mut(), b"collateral", &collateral_market);

        let cw20_debt_market_initial = th_init_market(
            deps.as_mut(),
            cw20_debt_contract_addr.as_bytes(),
            &cw20_debt_market,
        );

        let native_debt_market_initial =
            th_init_market(deps.as_mut(), b"native_debt", &native_debt_market);

        let mut expected_user_cw20_debt_scaled = compute_scaled_amount(
            user_debt,
            cw20_debt_market_initial.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();

        // Set user as having collateral and debt in respective markets
        {
            let mut user = User::default();
            set_bit(&mut user.collateral_assets, collateral_market_initial.index).unwrap();
            set_bit(&mut user.borrowed_assets, cw20_debt_market_initial.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &user_address, &user)
                .unwrap();
        }

        // trying to liquidate user with zero collateral balance should fail
        {
            deps.querier.set_cw20_balances(
                collateral_market_ma_token_addr.clone(),
                &[(user_address.clone(), Uint128::zero())],
            );

            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: first_debt_to_repay.into(),
            });

            let env = mock_env(MockEnvParams::default());
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::CannotLiquidateWhenNoCollateralBalance {}
            );
        }

        // Set the querier to return positive collateral balance
        deps.querier.set_cw20_balances(
            collateral_market_ma_token_addr.clone(),
            &[(
                user_address.clone(),
                Uint128::new(user_collateral_balance) * SCALING_FACTOR,
            )],
        );

        // trying to liquidate user with zero outstanding debt should fail (uncollateralized has not impact)
        {
            let debt = Debt {
                amount_scaled: Uint128::zero(),
                uncollateralized: false,
            };
            let uncollateralized_debt = Debt {
                amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
                uncollateralized: true,
            };
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                    &debt,
                )
                .unwrap();
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (b"uncollateralized_debt", &user_address),
                    &uncollateralized_debt,
                )
                .unwrap();

            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: first_debt_to_repay.into(),
            });

            let env = mock_env(MockEnvParams::default());
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::CannotLiquidateWhenNoDebtBalance {}
            );
        }

        // set user to have positive debt amount in debt asset
        {
            let debt = Debt {
                amount_scaled: expected_user_cw20_debt_scaled,
                uncollateralized: false,
            };
            let uncollateralized_debt = Debt {
                amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
                uncollateralized: true,
            };
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                    &debt,
                )
                .unwrap();
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (b"uncollateralized_debt", &user_address),
                    &uncollateralized_debt,
                )
                .unwrap();
        }

        // trying to liquidate without sending funds should fail
        {
            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: Uint128::zero(),
            });

            let env = mock_env(MockEnvParams::default());
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InvalidLiquidateAmount {
                    asset: "cw20_debt".to_string()
                }
            );
        }

        // trying to liquidate when collateral market inactive
        {
            let env = mock_env(MockEnvParams::default());
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: Uint128::new(100),
            });

            let mut collateral_market = MARKETS.load(&deps.storage, b"collateral").unwrap();
            collateral_market.active = false;
            MARKETS
                .save(&mut deps.storage, b"collateral", &collateral_market)
                .unwrap();

            let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::MarketNotActive {
                    asset: "collateral".to_string()
                }
            );

            collateral_market.active = true;
            MARKETS
                .save(&mut deps.storage, b"collateral", &collateral_market)
                .unwrap();
        }

        // trying to liquidate when debt market inactive
        {
            let env = mock_env(MockEnvParams::default());
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: Uint128::new(100),
            });

            let mut cw20_debt_market = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();
            cw20_debt_market.active = false;
            MARKETS
                .save(
                    &mut deps.storage,
                    cw20_debt_contract_addr.as_bytes(),
                    &cw20_debt_market,
                )
                .unwrap();

            let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::MarketNotActive {
                    asset: "cw20_debt".to_string()
                }
            );

            cw20_debt_market.active = true;
            MARKETS
                .save(
                    &mut deps.storage,
                    cw20_debt_contract_addr.as_bytes(),
                    &cw20_debt_market,
                )
                .unwrap();
        }

        // Perform first successful liquidation receiving ma_token in return
        {
            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: true,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: first_debt_to_repay.into(),
            });

            let collateral_market_before = MARKETS.load(&deps.storage, b"collateral").unwrap();
            let debt_market_before = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            let block_time = first_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

            // get expected indices and rates for debt market
            let expected_debt_rates = th_get_expected_indices_and_rates(
                &cw20_debt_market_initial,
                block_time,
                available_liquidity_cw20_debt,
                TestUtilizationDeltas {
                    less_debt: first_debt_to_repay.into(),
                    ..Default::default()
                },
            );

            let collateral_market_after = MARKETS.load(&deps.storage, b"collateral").unwrap();
            let debt_market_after = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            let expected_liquidated_collateral_amount = Decimal::divide_uint128_by_decimal(
                first_debt_to_repay
                    * cw20_debt_price
                    * (Decimal::one() + collateral_liquidation_bonus),
                collateral_price,
            )
            .unwrap();

            let expected_liquidated_collateral_amount_scaled = get_scaled_liquidity_amount(
                expected_liquidated_collateral_amount,
                &collateral_market_after,
                env.block.time.seconds(),
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collateral_market_ma_token_addr.to_string(),
                        msg: to_binary(
                            &mars_core::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                                sender: user_address.to_string(),
                                recipient: liquidator_address.to_string(),
                                amount: expected_liquidated_collateral_amount_scaled.into(),
                            }
                        )
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: cw20_debt_market.ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_debt_rates.protocol_rewards_to_distribute,
                                expected_debt_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                res.attributes,
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "collateral"),
                    attr("debt_asset", cw20_debt_contract_addr.as_str()),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_collateral_amount.to_string(),
                    ),
                    attr("debt_amount_repaid", first_debt_to_repay.to_string()),
                    attr("refund_amount", "0"),
                ],
            );
            assert_eq!(
                res.events,
                vec![
                    build_collateral_position_changed_event(
                        "collateral",
                        true,
                        liquidator_address.to_string()
                    ),
                    th_build_interests_updated_event(
                        cw20_debt_contract_addr.as_str(),
                        &expected_debt_rates
                    )
                ]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, collateral_market_before.index).unwrap());
            assert!(get_bit(user.borrowed_assets, debt_market_before.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let debt = DEBTS
                .load(
                    &deps.storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                )
                .unwrap();

            let expected_less_debt_scaled = compute_scaled_amount(
                first_debt_to_repay,
                expected_debt_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();

            expected_user_cw20_debt_scaled =
                expected_user_cw20_debt_scaled - expected_less_debt_scaled;

            assert_eq!(expected_user_cw20_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            expected_global_cw20_debt_scaled =
                expected_global_cw20_debt_scaled - expected_less_debt_scaled;

            assert_eq!(
                expected_global_cw20_debt_scaled,
                debt_market_after.debt_total_scaled
            );
        }

        // Perform second successful liquidation sending an excess amount (should refund)
        // and receive underlying collateral
        {
            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: false,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: second_debt_to_repay.into(),
            });

            let collateral_market_before = MARKETS.load(&deps.storage, b"collateral").unwrap();
            let debt_market_before = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            let block_time = second_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

            // get expected indices and rates for debt and collateral markets
            let expected_debt_indices = th_get_expected_indices(&debt_market_before, block_time);
            let user_debt_asset_total_debt = compute_underlying_amount(
                expected_user_cw20_debt_scaled,
                expected_debt_indices.borrow,
            )
            .unwrap();
            // Since debt is being over_repayed, we expect to max out the liquidatable debt
            let expected_less_debt = user_debt_asset_total_debt * close_factor;

            let expected_refund_amount = second_debt_to_repay - expected_less_debt;

            let expected_debt_rates = th_get_expected_indices_and_rates(
                &debt_market_before,
                block_time,
                available_liquidity_cw20_debt, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_debt: expected_less_debt.into(),
                    less_liquidity: expected_refund_amount.into(),
                    ..Default::default()
                },
            );

            let expected_liquidated_collateral_amount = Decimal::divide_uint128_by_decimal(
                expected_less_debt
                    * cw20_debt_price
                    * (Decimal::one() + collateral_liquidation_bonus),
                collateral_price,
            )
            .unwrap();

            let expected_collateral_rates = th_get_expected_indices_and_rates(
                &collateral_market_before,
                block_time,
                available_liquidity_collateral, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_liquidity: expected_liquidated_collateral_amount.into(),
                    ..Default::default()
                },
            );

            let debt_market_after = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
                expected_liquidated_collateral_amount,
                expected_collateral_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collateral_market_ma_token_addr.to_string(),
                        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
                            user: user_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![deduct_tax(
                            deps.as_ref(),
                            Coin {
                                denom: String::from("collateral"),
                                amount: expected_liquidated_collateral_amount,
                            }
                        )
                        .unwrap()],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collateral_market_ma_token_addr.to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_collateral_rates.protocol_rewards_to_distribute,
                                expected_collateral_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: cw20_debt_market.ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_debt_rates.protocol_rewards_to_distribute,
                                expected_debt_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: cw20_debt_contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: liquidator_address.to_string(),
                            amount: expected_refund_amount,
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "collateral"),
                    attr("debt_asset", cw20_debt_contract_addr.as_str()),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_collateral_amount,
                    ),
                    attr("debt_amount_repaid", expected_less_debt.to_string()),
                    attr("refund_amount", expected_refund_amount.to_string()),
                ],
                res.attributes,
            );
            assert_eq!(
                res.events,
                vec![
                    th_build_interests_updated_event("collateral", &expected_collateral_rates),
                    th_build_interests_updated_event(
                        cw20_debt_contract_addr.as_str(),
                        &expected_debt_rates
                    ),
                ]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, collateral_market_initial.index).unwrap());
            assert!(get_bit(user.borrowed_assets, cw20_debt_market_initial.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let expected_less_debt_scaled = compute_scaled_amount(
                expected_less_debt,
                expected_debt_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();
            expected_user_cw20_debt_scaled =
                expected_user_cw20_debt_scaled - expected_less_debt_scaled;

            let debt = DEBTS
                .load(
                    &deps.storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                )
                .unwrap();

            assert_eq!(expected_user_cw20_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            expected_global_cw20_debt_scaled =
                expected_global_cw20_debt_scaled - expected_less_debt_scaled;
            assert_eq!(
                expected_global_cw20_debt_scaled,
                debt_market_after.debt_total_scaled
            );
        }

        // Perform full liquidation receiving ma_token in return (user should not be able to use asset as collateral)
        {
            let user_collateral_balance_scaled = Uint128::new(100) * SCALING_FACTOR;
            let mut expected_user_debt_scaled = Uint128::new(400) * SCALING_FACTOR;
            let debt_to_repay = Uint128::from(300u128);

            // Set the querier to return positive collateral balance
            deps.querier.set_cw20_balances(
                collateral_market_ma_token_addr.clone(),
                &[(user_address.clone(), user_collateral_balance_scaled.into())],
            );

            // set user to have positive debt amount in debt asset
            let debt = Debt {
                amount_scaled: expected_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                    &debt,
                )
                .unwrap();

            let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                    collateral_asset: Asset::Native {
                        denom: "collateral".to_string(),
                    },
                    user_address: user_address.to_string(),
                    receive_ma_token: false,
                })
                .unwrap(),
                sender: liquidator_address.to_string(),
                amount: debt_to_repay.into(),
            });

            let collateral_market_before = MARKETS.load(&deps.storage, b"collateral").unwrap();
            let debt_market_before = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            let block_time = second_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = mock_info(cw20_debt_contract_addr.as_str());
            let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

            // get expected indices and rates for debt and collateral markets
            let expected_collateral_indices =
                th_get_expected_indices(&collateral_market_before, block_time);
            let user_collateral_balance = compute_underlying_amount(
                user_collateral_balance_scaled,
                expected_collateral_indices.liquidity,
            )
            .unwrap();

            // Since debt is being over_repayed, we expect to liquidate total collateral
            let expected_less_debt = Decimal::divide_uint128_by_decimal(
                Decimal::divide_uint128_by_decimal(
                    collateral_price * user_collateral_balance,
                    cw20_debt_price,
                )
                .unwrap(),
                Decimal::one() + collateral_liquidation_bonus,
            )
            .unwrap();

            let expected_refund_amount = debt_to_repay - expected_less_debt;

            let expected_debt_rates = th_get_expected_indices_and_rates(
                &debt_market_before,
                block_time,
                available_liquidity_cw20_debt, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_debt: expected_less_debt.into(),
                    less_liquidity: expected_refund_amount.into(),
                    ..Default::default()
                },
            );

            let expected_collateral_rates = th_get_expected_indices_and_rates(
                &collateral_market_before,
                block_time,
                available_liquidity_collateral, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_liquidity: user_collateral_balance.into(),
                    ..Default::default()
                },
            );

            let debt_market_after = MARKETS
                .load(&deps.storage, cw20_debt_contract_addr.as_bytes())
                .unwrap();

            // NOTE: expected_liquidated_collateral_amount_scaled should be equal user_collateral_balance_scaled
            // but there are rounding errors
            let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
                user_collateral_balance,
                expected_collateral_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collateral_market_ma_token_addr.to_string(),
                        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
                            user: user_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![deduct_tax(
                            deps.as_ref(),
                            Coin {
                                denom: String::from("collateral"),
                                amount: user_collateral_balance,
                            }
                        )
                        .unwrap()],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: cw20_debt_contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: liquidator_address.to_string(),
                            amount: expected_refund_amount,
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "collateral"),
                    attr("debt_asset", cw20_debt_contract_addr.as_str()),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        user_collateral_balance.to_string(),
                    ),
                    attr("debt_amount_repaid", expected_less_debt.to_string()),
                    attr("refund_amount", expected_refund_amount.to_string()),
                ],
                res.attributes,
            );
            assert_eq!(
                res.events,
                vec![
                    build_collateral_position_changed_event(
                        "collateral",
                        false,
                        user_address.to_string()
                    ),
                    th_build_interests_updated_event("collateral", &expected_collateral_rates),
                    th_build_interests_updated_event(
                        cw20_debt_contract_addr.as_str(),
                        &expected_debt_rates
                    ),
                ]
            );

            // check user doesn't have deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(!get_bit(user.collateral_assets, collateral_market_initial.index).unwrap());
            assert!(get_bit(user.borrowed_assets, cw20_debt_market_initial.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let expected_less_debt_scaled = compute_scaled_amount(
                expected_less_debt,
                expected_debt_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();
            expected_user_debt_scaled = expected_user_debt_scaled - expected_less_debt_scaled;

            let debt = DEBTS
                .load(
                    &deps.storage,
                    (cw20_debt_contract_addr.as_bytes(), &user_address),
                )
                .unwrap();

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            expected_global_cw20_debt_scaled =
                expected_global_cw20_debt_scaled - expected_less_debt_scaled;
            assert_eq!(
                expected_global_cw20_debt_scaled,
                debt_market_after.debt_total_scaled
            );
        }

        // send many native coins
        {
            let env = mock_env(MockEnvParams::default());
            let info = cosmwasm_std::testing::mock_info(
                "liquidator",
                &[coin(100, "somecoin1"), coin(200, "somecoin2")],
            );
            let msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "collateral".to_string(),
                },
                debt_asset_denom: "somecoin2".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: false,
            };
            let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::InvalidNativeCoinsSent {
                    denom: "somecoin2".to_string()
                }
            );
        }

        // Perform native liquidation receiving ma_token in return
        {
            let mut user = User::default();
            set_bit(&mut user.collateral_assets, collateral_market_initial.index).unwrap();
            set_bit(&mut user.borrowed_assets, native_debt_market_initial.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &user_address, &user)
                .unwrap();

            let user_collateral_balance_scaled = Uint128::new(200) * SCALING_FACTOR;
            let mut expected_user_debt_scaled = Uint128::new(800) * SCALING_FACTOR;
            let debt_to_repay = Uint128::from(500u128);

            // Set the querier to return positive collateral balance
            deps.querier.set_cw20_balances(
                Addr::unchecked("ma_collateral"),
                &[(user_address.clone(), user_collateral_balance_scaled.into())],
            );

            // set user to have positive debt amount in debt asset
            let debt = Debt {
                amount_scaled: expected_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (b"native_debt", &user_address),
                    &debt,
                )
                .unwrap();

            let liquidate_msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "collateral".to_string(),
                },
                debt_asset_denom: "native_debt".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: false,
            };

            let collateral_market_before = MARKETS.load(&deps.storage, b"collateral").unwrap();
            let debt_market_before = MARKETS.load(&deps.storage, b"native_debt").unwrap();

            let block_time = second_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = cosmwasm_std::testing::mock_info(
                liquidator_address.as_str(),
                &[coin(debt_to_repay.u128(), "native_debt")],
            );
            let res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap();

            // get expected indices and rates for debt and collateral markets
            let expected_collateral_indices =
                th_get_expected_indices(&collateral_market_before, block_time);
            let user_collateral_balance = compute_underlying_amount(
                user_collateral_balance_scaled,
                expected_collateral_indices.liquidity,
            )
            .unwrap();

            // Since debt is being over_repayed, we expect to liquidate total collateral
            let expected_less_debt = Decimal::divide_uint128_by_decimal(
                Decimal::divide_uint128_by_decimal(
                    collateral_price * user_collateral_balance,
                    native_debt_price,
                )
                .unwrap(),
                Decimal::one() + collateral_liquidation_bonus,
            )
            .unwrap();

            let expected_refund_amount = debt_to_repay - expected_less_debt;

            let expected_debt_rates = th_get_expected_indices_and_rates(
                &debt_market_before,
                block_time,
                available_liquidity_native_debt, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_debt: expected_less_debt.into(),
                    less_liquidity: expected_refund_amount.into(),
                    ..Default::default()
                },
            );

            let expected_collateral_rates = th_get_expected_indices_and_rates(
                &collateral_market_before,
                block_time,
                available_liquidity_collateral, // this is the same as before as it comes from mocks
                TestUtilizationDeltas {
                    less_liquidity: user_collateral_balance.into(),
                    ..Default::default()
                },
            );

            let debt_market_after = MARKETS.load(&deps.storage, b"native_debt").unwrap();

            // NOTE: expected_liquidated_collateral_amount_scaled should be equal user_collateral_balance_scaled
            // but there are rounding errors
            let expected_liquidated_collateral_amount_scaled = compute_scaled_amount(
                user_collateral_balance,
                expected_collateral_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            // no rewards to distribute for collateral asset, so no mint message is
            // sent
            assert_eq!(
                expected_collateral_rates.protocol_rewards_to_distribute,
                Uint128::zero()
            );

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collateral_market_ma_token_addr.to_string(),
                        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
                            user: user_address.to_string(),
                            amount: expected_liquidated_collateral_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![deduct_tax(
                            deps.as_ref(),
                            Coin {
                                denom: String::from("collateral"),
                                amount: user_collateral_balance,
                            }
                        )
                        .unwrap()],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: debt_market_after.ma_token_address.to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_debt_rates.protocol_rewards_to_distribute,
                                expected_debt_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![deduct_tax(
                            deps.as_ref(),
                            Coin {
                                denom: String::from("native_debt"),
                                amount: expected_refund_amount,
                            }
                        )
                        .unwrap()],
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "collateral"),
                    attr("debt_asset", "native_debt"),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        user_collateral_balance.to_string(),
                    ),
                    attr("debt_amount_repaid", expected_less_debt.to_string()),
                    attr("refund_amount", expected_refund_amount.to_string()),
                ],
                res.attributes,
            );
            assert_eq!(
                res.events,
                vec![
                    build_collateral_position_changed_event(
                        "collateral",
                        false,
                        user_address.to_string()
                    ),
                    th_build_interests_updated_event("collateral", &expected_collateral_rates),
                    th_build_interests_updated_event("native_debt", &expected_debt_rates),
                ]
            );

            // check user doesn't have deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(!get_bit(user.collateral_assets, collateral_market_initial.index).unwrap());
            assert!(get_bit(user.borrowed_assets, native_debt_market_initial.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let expected_less_debt_scaled = compute_scaled_amount(
                expected_less_debt,
                expected_debt_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();
            expected_user_debt_scaled = expected_user_debt_scaled - expected_less_debt_scaled;

            let debt = DEBTS
                .load(&deps.storage, (b"native_debt", &user_address))
                .unwrap();

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            expected_global_native_debt_scaled =
                expected_global_native_debt_scaled - expected_less_debt_scaled;
            assert_eq!(
                expected_global_native_debt_scaled,
                debt_market_after.debt_total_scaled
            );
        }
    }

    #[test]
    fn test_liquidate_with_same_asset_for_debt_and_collateral() {
        // Setup
        let available_liquidity = 1_000_000_000u128;
        let mut deps = th_setup(&[coin(available_liquidity, "the_asset")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::zero(),
            &[(String::from("the_asset"), Uint128::new(100u128))],
        );

        let user_address = Addr::unchecked("user");
        let liquidator_address = Addr::unchecked("liquidator");
        let ma_token_address = Addr::unchecked("mathe_asset");

        let asset_max_ltv = Decimal::from_ratio(5u128, 10u128);
        let asset_liquidation_threshold = Decimal::from_ratio(6u128, 10u128);
        let asset_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);
        let asset_price = Decimal::from_ratio(2_u128, 1_u128);

        let initial_user_debt_balance = Uint128::from(3_000_000_u64);
        // NOTE: this should change in practice but it will stay static on this test
        // as the balance is mocked and does not get updated
        let user_collateral_balance = Uint128::from(2_000_000_u64);

        let close_factor = Decimal::from_ratio(1u128, 2u128);

        // Global debt for the market (starts at index 1.000000000...)
        let initial_global_debt_scaled = Uint128::new(500_000_000) * SCALING_FACTOR;
        let liquidation_block_time = 15_000_000;

        CONFIG
            .update(deps.as_mut().storage, |mut config| -> StdResult<_> {
                config.close_factor = close_factor;
                Ok(config)
            })
            .unwrap();

        // initialize market
        deps.querier
            .set_oracle_price(b"the_asset".to_vec(), asset_price);

        let interest_rate_params = LinearInterestRateModelParams {
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            base: Decimal::from_ratio(0u128, 100u128),
            slope_1: Decimal::from_ratio(10u128, 100u128),
            slope_2: Decimal::one(),
        };

        let asset_market = Market {
            ma_token_address: ma_token_address.clone(),
            max_loan_to_value: asset_max_ltv,
            liquidation_threshold: asset_liquidation_threshold,
            liquidation_bonus: asset_liquidation_bonus,
            debt_total_scaled: initial_global_debt_scaled,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::from_ratio(2u128, 10u128),
            liquidity_rate: Decimal::from_ratio(2u128, 10u128),
            reserve_factor: Decimal::from_ratio(2u128, 100u128),
            asset_type: AssetType::Native,
            indexes_last_updated: 0,
            interest_rate_model: InterestRateModel::Linear {
                params: interest_rate_params.clone(),
            },
            ..Default::default()
        };

        let asset_market_initial = th_init_market(deps.as_mut(), b"the_asset", &asset_market);

        let initial_user_debt_scaled = compute_scaled_amount(
            initial_user_debt_balance,
            asset_market_initial.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();

        // Set user as having collateral and debt in market
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, asset_market_initial.index).unwrap();
        set_bit(&mut user.borrowed_assets, asset_market_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &user_address, &user)
            .unwrap();

        // Set the querier to return positive collateral balance
        deps.querier.set_cw20_balances(
            ma_token_address.clone(),
            &[(
                user_address.clone(),
                user_collateral_balance * SCALING_FACTOR,
            )],
        );

        // set user to have positive debt amount in debt asset
        {
            let debt = Debt {
                amount_scaled: initial_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"the_asset", &user_address), &debt)
                .unwrap();
        }

        // Perform partial liquidation receiving ma_token in return
        {
            let debt_to_repay = Uint128::from(400_000_u64);
            let liquidate_msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "the_asset".to_string(),
                },
                debt_asset_denom: "the_asset".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: true,
            };

            let asset_market_before = MARKETS.load(&deps.storage, b"the_asset").unwrap();

            let block_time = liquidation_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = cosmwasm_std::testing::mock_info(
                liquidator_address.as_str(),
                &[coin(debt_to_repay.into(), "the_asset")],
            );
            let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

            // get expected indices and rates for debt market
            let expected_rates = th_get_expected_indices_and_rates(
                &asset_market_before,
                block_time,
                available_liquidity,
                TestUtilizationDeltas {
                    less_debt: debt_to_repay.into(),
                    ..Default::default()
                },
            );

            let asset_market_after = MARKETS.load(&deps.storage, b"the_asset").unwrap();

            let expected_liquidated_amount = Decimal::divide_uint128_by_decimal(
                debt_to_repay * asset_price * (Decimal::one() + asset_liquidation_bonus),
                asset_price,
            )
            .unwrap();

            let expected_liquidated_amount_scaled = get_scaled_liquidity_amount(
                expected_liquidated_amount,
                &asset_market_after,
                env.block.time.seconds(),
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.to_string(),
                        msg: to_binary(
                            &mars_core::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                                sender: user_address.to_string(),
                                recipient: liquidator_address.to_string(),
                                amount: expected_liquidated_amount_scaled.into(),
                            }
                        )
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_rates.protocol_rewards_to_distribute,
                                expected_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                res.attributes,
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "the_asset"),
                    attr("debt_asset", "the_asset"),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_amount.to_string(),
                    ),
                    attr("debt_amount_repaid", debt_to_repay.to_string()),
                    attr("refund_amount", "0"),
                ],
            );
            assert_eq!(
                res.events,
                vec![
                    build_collateral_position_changed_event(
                        "the_asset",
                        true,
                        liquidator_address.to_string()
                    ),
                    th_build_interests_updated_event("the_asset", &expected_rates)
                ]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
            assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

            // check liquidator gets its collateral bit set
            let liquidator = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(liquidator.collateral_assets, asset_market_before.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let debt = DEBTS
                .load(&deps.storage, (b"the_asset", &user_address))
                .unwrap();

            let expected_less_debt_scaled = compute_scaled_amount(
                debt_to_repay.into(),
                expected_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();

            let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            let expected_global_debt_scaled =
                initial_global_debt_scaled - expected_less_debt_scaled;

            assert_eq!(
                expected_global_debt_scaled,
                asset_market_after.debt_total_scaled
            );
        }

        // Reset state for next test
        {
            let debt = Debt {
                amount_scaled: initial_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"the_asset", &user_address), &debt)
                .unwrap();

            MARKETS
                .save(deps.as_mut().storage, b"the_asset", &asset_market_initial)
                .unwrap();

            // NOTE: Do not reset liquidator in order to check that position is not reset in next
            // liquidation receiving ma tokens
        }

        // Perform partial liquidation receiving underlying asset in return
        {
            let debt_to_repay = Uint128::from(400_000_u64);
            let liquidate_msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "the_asset".to_string(),
                },
                debt_asset_denom: "the_asset".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: false,
            };

            let asset_market_before = MARKETS.load(&deps.storage, b"the_asset").unwrap();

            let block_time = liquidation_block_time;
            let env = mock_env_at_block_time(block_time);
            let info = cosmwasm_std::testing::mock_info(
                liquidator_address.as_str(),
                &[coin(debt_to_repay.into(), "the_asset")],
            );
            let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

            let asset_market_after = MARKETS.load(&deps.storage, b"the_asset").unwrap();
            let expected_liquidated_amount = Decimal::divide_uint128_by_decimal(
                debt_to_repay * asset_price * (Decimal::one() + asset_liquidation_bonus),
                asset_price,
            )
            .unwrap();

            // get expected indices and rates for debt market
            let expected_rates = th_get_expected_indices_and_rates(
                &asset_market_before,
                block_time,
                available_liquidity,
                TestUtilizationDeltas {
                    less_debt: debt_to_repay.into(),
                    less_liquidity: expected_liquidated_amount.into(),
                    ..Default::default()
                },
            );

            let expected_liquidated_amount_scaled = compute_scaled_amount(
                expected_liquidated_amount,
                expected_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.to_string(),
                        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
                            user: user_address.to_string(),
                            amount: expected_liquidated_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    // NOTE: Tax set to 0 so no tax should be charged
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![Coin {
                            denom: String::from("the_asset"),
                            amount: expected_liquidated_amount,
                        }],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_rates.protocol_rewards_to_distribute,
                                expected_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                res.attributes,
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "the_asset"),
                    attr("debt_asset", "the_asset"),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_amount.to_string(),
                    ),
                    attr("debt_amount_repaid", debt_to_repay.to_string()),
                    attr("refund_amount", "0"),
                ],
            );
            assert_eq!(
                res.events,
                vec![th_build_interests_updated_event(
                    "the_asset",
                    &expected_rates
                ),]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
            assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let debt = DEBTS
                .load(&deps.storage, (b"the_asset", &user_address))
                .unwrap();

            let expected_less_debt_scaled = compute_scaled_amount(
                debt_to_repay.into(),
                expected_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();

            let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            let expected_global_debt_scaled =
                initial_global_debt_scaled - expected_less_debt_scaled;

            assert_eq!(
                expected_global_debt_scaled,
                asset_market_after.debt_total_scaled
            );
        }

        // Reset state for next test
        {
            let debt = Debt {
                amount_scaled: initial_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"the_asset", &user_address), &debt)
                .unwrap();

            MARKETS
                .save(deps.as_mut().storage, b"the_asset", &asset_market_initial)
                .unwrap();

            // NOTE: Do not reset liquidator having the asset as collateral in order to check
            // position changed event is not emitted
        }

        // Perform overpaid liquidation receiving ma_token in return
        {
            let block_time = liquidation_block_time;
            // Since debt is being over repayed, we expect to max out the liquidatable debt
            // get expected indices and rates for debt and collateral markets
            let expected_indices = th_get_expected_indices(&asset_market_initial, block_time);
            let user_debt_balance_before =
                compute_underlying_amount(initial_user_debt_scaled, expected_indices.borrow)
                    .unwrap();
            let debt_to_repay = user_debt_balance_before;
            let expected_less_debt = user_debt_balance_before * close_factor;
            let expected_refund_amount = debt_to_repay - expected_less_debt;

            let liquidate_msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "the_asset".to_string(),
                },
                debt_asset_denom: "the_asset".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: true,
            };

            let asset_market_before = MARKETS.load(&deps.storage, b"the_asset").unwrap();

            let env = mock_env_at_block_time(block_time);
            let info = cosmwasm_std::testing::mock_info(
                liquidator_address.as_str(),
                &[coin(debt_to_repay.into(), "the_asset")],
            );
            let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

            let asset_market_after = MARKETS.load(&deps.storage, b"the_asset").unwrap();
            let expected_liquidated_amount = Decimal::divide_uint128_by_decimal(
                expected_less_debt * asset_price * (Decimal::one() + asset_liquidation_bonus),
                asset_price,
            )
            .unwrap();

            // get expected indices and rates for debt market
            let expected_rates = th_get_expected_indices_and_rates(
                &asset_market_before,
                block_time,
                available_liquidity,
                TestUtilizationDeltas {
                    less_debt: expected_less_debt.into(),
                    less_liquidity: expected_refund_amount.into(),
                    ..Default::default()
                },
            );

            let expected_liquidated_amount_scaled = compute_scaled_amount(
                expected_liquidated_amount,
                expected_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.to_string(),
                        msg: to_binary(
                            &mars_core::ma_token::msg::ExecuteMsg::TransferOnLiquidation {
                                sender: user_address.to_string(),
                                recipient: liquidator_address.to_string(),
                                amount: expected_liquidated_amount_scaled.into(),
                            }
                        )
                        .unwrap(),
                        funds: vec![]
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_rates.protocol_rewards_to_distribute,
                                expected_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    // NOTE: Tax set to 0 so no tax should be charged
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![Coin {
                            denom: String::from("the_asset"),
                            amount: expected_refund_amount,
                        }],
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                res.attributes,
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "the_asset"),
                    attr("debt_asset", "the_asset"),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_amount.to_string(),
                    ),
                    attr("debt_amount_repaid", expected_less_debt.to_string()),
                    attr("refund_amount", expected_refund_amount),
                ],
            );
            assert_eq!(
                res.events,
                vec![
                    th_build_interests_updated_event("the_asset", &expected_rates),
                    // NOTE: Should not emit position change event as it was changed on the
                    // first call and was not reset
                ]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
            assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

            // check user's debt decreased by the appropriate amount
            let debt = DEBTS
                .load(&deps.storage, (b"the_asset", &user_address))
                .unwrap();

            let expected_less_debt_scaled = compute_scaled_amount(
                expected_less_debt.into(),
                expected_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();

            let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            let expected_global_debt_scaled =
                initial_global_debt_scaled - expected_less_debt_scaled;

            assert_eq!(
                expected_global_debt_scaled,
                asset_market_after.debt_total_scaled
            );
        }

        // Reset state for next test
        {
            let debt = Debt {
                amount_scaled: initial_user_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"the_asset", &user_address), &debt)
                .unwrap();

            MARKETS
                .save(deps.as_mut().storage, b"the_asset", &asset_market_initial)
                .unwrap();

            // NOTE: reset liquidator to not having the asset as collateral in order to check
            // position is not changed when receiving underlying asset
            let liquidator = User::default();
            USERS
                .save(deps.as_mut().storage, &liquidator_address, &liquidator)
                .unwrap();
        }

        // Perform overpaid liquidation receiving underlying asset in return
        {
            let block_time = liquidation_block_time;
            // Since debt is being over repayed, we expect to max out the liquidatable debt
            // get expected indices and rates for debt and collateral markets
            let expected_indices = th_get_expected_indices(&asset_market_initial, block_time);
            let user_debt_balance_before =
                compute_underlying_amount(initial_user_debt_scaled, expected_indices.borrow)
                    .unwrap();
            let debt_to_repay = user_debt_balance_before;
            let expected_less_debt = user_debt_balance_before * close_factor;
            let expected_refund_amount = debt_to_repay - expected_less_debt;

            let liquidate_msg = ExecuteMsg::LiquidateNative {
                collateral_asset: Asset::Native {
                    denom: "the_asset".to_string(),
                },
                debt_asset_denom: "the_asset".to_string(),
                user_address: user_address.to_string(),
                receive_ma_token: false,
            };

            let asset_market_before = MARKETS.load(&deps.storage, b"the_asset").unwrap();

            let env = mock_env_at_block_time(block_time);
            let info = cosmwasm_std::testing::mock_info(
                liquidator_address.as_str(),
                &[coin(debt_to_repay.into(), "the_asset")],
            );
            let res = execute(deps.as_mut(), env.clone(), info, liquidate_msg).unwrap();

            let asset_market_after = MARKETS.load(&deps.storage, b"the_asset").unwrap();
            let expected_liquidated_amount = Decimal::divide_uint128_by_decimal(
                expected_less_debt * asset_price * (Decimal::one() + asset_liquidation_bonus),
                asset_price,
            )
            .unwrap();

            // get expected indices and rates for debt market
            let expected_rates = th_get_expected_indices_and_rates(
                &asset_market_before,
                block_time,
                available_liquidity,
                TestUtilizationDeltas {
                    less_debt: expected_less_debt.into(),
                    less_liquidity: (expected_refund_amount + expected_liquidated_amount).into(),
                    ..Default::default()
                },
            );

            let expected_liquidated_amount_scaled = compute_scaled_amount(
                expected_liquidated_amount,
                expected_rates.liquidity_index,
                ScalingOperation::Liquidity,
            )
            .unwrap();

            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.to_string(),
                        msg: to_binary(&mars_core::ma_token::msg::ExecuteMsg::Burn {
                            user: user_address.to_string(),
                            amount: expected_liquidated_amount_scaled.into(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    // NOTE: Tax set to 0 so no tax should be charged
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![Coin {
                            denom: String::from("the_asset"),
                            amount: expected_liquidated_amount,
                        }],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: ma_token_address.clone().to_string(),
                        msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                            recipient: "protocol_rewards_collector".to_string(),
                            amount: compute_scaled_amount(
                                expected_rates.protocol_rewards_to_distribute,
                                expected_rates.liquidity_index,
                                ScalingOperation::Liquidity
                            )
                            .unwrap(),
                        })
                        .unwrap(),
                        funds: vec![]
                    })),
                    // NOTE: Tax set to 0 so no tax should be charged
                    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: liquidator_address.to_string(),
                        amount: vec![Coin {
                            denom: String::from("the_asset"),
                            amount: expected_refund_amount,
                        }],
                    })),
                ]
            );

            mars_core::testing::assert_eq_vec(
                res.attributes,
                vec![
                    attr("action", "liquidate"),
                    attr("collateral_asset", "the_asset"),
                    attr("debt_asset", "the_asset"),
                    attr("user", user_address.as_str()),
                    attr("liquidator", liquidator_address.as_str()),
                    attr(
                        "collateral_amount_liquidated",
                        expected_liquidated_amount.to_string(),
                    ),
                    attr("debt_amount_repaid", expected_less_debt.to_string()),
                    attr("refund_amount", expected_refund_amount),
                ],
            );
            assert_eq!(
                res.events,
                vec![th_build_interests_updated_event(
                    "the_asset",
                    &expected_rates
                ),]
            );

            // check user still has deposited collateral asset and
            // still has outstanding debt in debt asset
            let user = USERS.load(&deps.storage, &user_address).unwrap();
            assert!(get_bit(user.collateral_assets, asset_market_before.index).unwrap());
            assert!(get_bit(user.borrowed_assets, asset_market_before.index).unwrap());

            // check liquidator does not have collateral bit set
            let liquidator = USERS.load(&deps.storage, &liquidator_address).unwrap();
            assert!(!(get_bit(liquidator.collateral_assets, asset_market_before.index).unwrap()));

            // check user's debt decreased by the appropriate amount
            let debt = DEBTS
                .load(&deps.storage, (b"the_asset", &user_address))
                .unwrap();

            let expected_less_debt_scaled = compute_scaled_amount(
                expected_less_debt.into(),
                expected_rates.borrow_index,
                ScalingOperation::Debt,
            )
            .unwrap();

            let expected_user_debt_scaled = initial_user_debt_scaled - expected_less_debt_scaled;

            assert_eq!(expected_user_debt_scaled, debt.amount_scaled);

            // check global debt decreased by the appropriate amount
            let expected_global_debt_scaled =
                initial_global_debt_scaled - expected_less_debt_scaled;

            assert_eq!(
                expected_global_debt_scaled,
                asset_market_after.debt_total_scaled
            );
        }
    }

    #[test]
    fn test_underlying_asset_balance_check_when_transfer_to_liquidator() {
        let native_collateral_liquidity = 4510000u128;
        let mut deps = th_setup(&[coin(native_collateral_liquidity, "native_collateral")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("native_collateral"), Uint128::new(100u128))],
        );

        let cw20_collateral_liquidity = 6510000u128;
        let cw20_collateral_contract_addr = Addr::unchecked("cw20_collateral");
        deps.querier.set_cw20_balances(
            cw20_collateral_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(cw20_collateral_liquidity),
            )],
        );

        let user_addr = Addr::unchecked("user");
        let liquidator_addr = Addr::unchecked("liquidator");
        let env = mock_env(MockEnvParams::default());

        // Indices changed in order to detect that there is no scaling on asset balance
        let market = Market {
            liquidity_index: Decimal::from_ratio(2u128, 1u128),
            borrow_index: Decimal::from_ratio(4u128, 1u128),
            asset_type: AssetType::Native,
            ..Default::default()
        };

        {
            // Trying to transfer more underlying native asset than available should fail
            let collateral_amount_to_liquidate = Uint128::new(native_collateral_liquidity + 1u128);
            let error_res = process_underlying_asset_transfer_to_liquidator(
                deps.as_mut(),
                &env,
                &user_addr,
                &liquidator_addr,
                "native_collateral".to_string(),
                AssetType::Native,
                &market,
                collateral_amount_to_liquidate,
                Response::new(),
            )
            .unwrap_err();
            assert_eq!(
                error_res,
                ContractError::CannotLiquidateWhenNotEnoughCollateral {}
            );
        }

        {
            // Trying to transfer less underlying native asset than available should pass
            let collateral_amount_to_liquidate = Uint128::new(native_collateral_liquidity - 1u128);
            let _res = process_underlying_asset_transfer_to_liquidator(
                deps.as_mut(),
                &env,
                &user_addr,
                &liquidator_addr,
                "native_collateral".to_string(),
                AssetType::Native,
                &market,
                collateral_amount_to_liquidate,
                Response::new(),
            )
            .unwrap();
        }

        // Indices changed in order to detect that there is no scaling on asset balance
        let market = Market {
            liquidity_index: Decimal::from_ratio(8u128, 1u128),
            borrow_index: Decimal::from_ratio(6u128, 1u128),
            asset_type: AssetType::Cw20,
            ..Default::default()
        };

        {
            // Trying to transfer more underlying cw20 asset than available should fail
            let collateral_amount_to_liquidate = Uint128::new(cw20_collateral_liquidity + 1u128);
            let error_res = process_underlying_asset_transfer_to_liquidator(
                deps.as_mut(),
                &env,
                &user_addr,
                &liquidator_addr,
                "cw20_collateral".to_string(),
                AssetType::Cw20,
                &market,
                collateral_amount_to_liquidate,
                Response::new(),
            )
            .unwrap_err();
            assert_eq!(
                error_res,
                ContractError::CannotLiquidateWhenNotEnoughCollateral {}
            );
        }

        {
            // Trying to transfer less underlying cw20 asset than available should pass
            let collateral_amount_to_liquidate = Uint128::new(cw20_collateral_liquidity - 1u128);
            let _res = process_underlying_asset_transfer_to_liquidator(
                deps.as_mut(),
                &env,
                &user_addr,
                &liquidator_addr,
                "cw20_collateral".to_string(),
                AssetType::Cw20,
                &market,
                collateral_amount_to_liquidate,
                Response::new(),
            )
            .unwrap();
        }
    }

    #[test]
    fn test_liquidation_health_factor_check() {
        // initialize collateral and debt markets
        let available_liquidity_collateral = 1000000000u128;
        let available_liquidity_debt = 2000000000u128;
        let mut deps = th_setup(&[coin(available_liquidity_collateral, "collateral")]);

        let debt_contract_addr = Addr::unchecked("debt");
        deps.querier.set_cw20_balances(
            debt_contract_addr.clone(),
            &[(
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                Uint128::new(available_liquidity_debt),
            )],
        );

        deps.querier
            .set_oracle_price(b"collateral".to_vec(), Decimal::one());
        deps.querier
            .set_oracle_price(b"debt".to_vec(), Decimal::one());

        let collateral_ltv = Decimal::from_ratio(5u128, 10u128);
        let collateral_liquidation_threshold = Decimal::from_ratio(7u128, 10u128);
        let collateral_liquidation_bonus = Decimal::from_ratio(1u128, 10u128);

        let collateral_market = Market {
            ma_token_address: Addr::unchecked("collateral"),
            max_loan_to_value: collateral_ltv,
            liquidation_threshold: collateral_liquidation_threshold,
            liquidation_bonus: collateral_liquidation_bonus,
            debt_total_scaled: Uint128::zero(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let debt_market = Market {
            ma_token_address: Addr::unchecked("debt"),
            max_loan_to_value: Decimal::from_ratio(6u128, 10u128),
            debt_total_scaled: Uint128::new(20_000_000) * SCALING_FACTOR,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            asset_type: AssetType::Cw20,
            ..Default::default()
        };

        // initialize markets
        let collateral_market_initial =
            th_init_market(deps.as_mut(), b"collateral", &collateral_market);

        let debt_market_initial =
            th_init_market(deps.as_mut(), debt_contract_addr.as_bytes(), &debt_market);

        // test health factor check
        let healthy_user_address = Addr::unchecked("healthy_user");

        // Set user as having collateral and debt in respective markets
        let mut healthy_user = User::default();

        set_bit(
            &mut healthy_user.collateral_assets,
            collateral_market_initial.index,
        )
        .unwrap();
        set_bit(&mut healthy_user.borrowed_assets, debt_market_initial.index).unwrap();

        USERS
            .save(deps.as_mut().storage, &healthy_user_address, &healthy_user)
            .unwrap();

        // set initial collateral and debt balances for user
        let collateral_address = Addr::unchecked("collateral");
        let healthy_user_collateral_balance_scaled = Uint128::new(10_000_000) * SCALING_FACTOR;

        // Set the querier to return a certain collateral balance
        deps.querier.set_cw20_balances(
            collateral_address,
            &[(
                healthy_user_address.clone(),
                healthy_user_collateral_balance_scaled.into(),
            )],
        );

        let healthy_user_debt_amount_scaled =
            Uint128::new(healthy_user_collateral_balance_scaled.u128())
                * collateral_liquidation_threshold;
        let healthy_user_debt = Debt {
            amount_scaled: healthy_user_debt_amount_scaled.into(),
            uncollateralized: false,
        };
        let uncollateralized_debt = Debt {
            amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
            uncollateralized: true,
        };
        DEBTS
            .save(
                deps.as_mut().storage,
                (debt_contract_addr.as_bytes(), &healthy_user_address),
                &healthy_user_debt,
            )
            .unwrap();
        DEBTS
            .save(
                deps.as_mut().storage,
                (b"uncollateralized_debt", &healthy_user_address),
                &uncollateralized_debt,
            )
            .unwrap();

        // perform liquidation (should fail because health factor is > 1)
        let liquidator_address = Addr::unchecked("liquidator");
        let debt_to_cover = Uint128::from(1_000_000u64);

        let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                collateral_asset: Asset::Native {
                    denom: "collateral".to_string(),
                },
                user_address: healthy_user_address.to_string(),
                receive_ma_token: true,
            })
            .unwrap(),
            sender: liquidator_address.to_string(),
            amount: debt_to_cover,
        });

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(debt_contract_addr.as_str());
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(error_res, ContractError::CannotLiquidateHealthyPosition {});
    }

    #[test]
    fn test_liquidate_if_collateral_disabled() {
        // initialize collateral and debt markets
        let mut deps = th_setup(&[]);

        let debt_contract_addr = Addr::unchecked("debt");

        let collateral_market_1 = Market {
            ma_token_address: Addr::unchecked("collateral1"),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let collateral_market_2 = Market {
            ma_token_address: Addr::unchecked("collateral2"),
            asset_type: AssetType::Native,
            ..Default::default()
        };
        let debt_market = Market {
            ma_token_address: Addr::unchecked("debt"),
            asset_type: AssetType::Cw20,
            ..Default::default()
        };

        // initialize markets
        let collateral_market_initial_1 =
            th_init_market(deps.as_mut(), b"collateral1", &collateral_market_1);
        let _collateral_market_initial_2 =
            th_init_market(deps.as_mut(), b"collateral2", &collateral_market_2);

        let debt_market_initial =
            th_init_market(deps.as_mut(), debt_contract_addr.as_bytes(), &debt_market);

        // Set user as having collateral and debt in respective markets
        let user_address = Addr::unchecked("user");
        let mut user = User::default();
        set_bit(
            &mut user.collateral_assets,
            collateral_market_initial_1.index,
        )
        .unwrap();
        set_bit(&mut user.borrowed_assets, debt_market_initial.index).unwrap();

        USERS
            .save(deps.as_mut().storage, &user_address, &user)
            .unwrap();

        // perform liquidation (should fail because collateral2 isn't set as collateral for user)
        let liquidator_address = Addr::unchecked("liquidator");
        let debt_to_cover = Uint128::from(1_000_000u64);

        let liquidate_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            msg: to_binary(&ReceiveMsg::LiquidateCw20 {
                collateral_asset: Asset::Native {
                    denom: "collateral2".to_string(),
                },
                user_address: user_address.to_string(),
                receive_ma_token: true,
            })
            .unwrap(),
            sender: liquidator_address.to_string(),
            amount: debt_to_cover,
        });

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(debt_contract_addr.as_str());
        let error_res = execute(deps.as_mut(), env, info, liquidate_msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::CannotLiquidateWhenCollateralUnset {
                asset: "collateral2".to_string()
            }
        );
    }

    #[test]
    fn test_finalize_liquidity_token_transfer() {
        // Setup
        let mut deps = th_setup(&[]);
        let env = mock_env(MockEnvParams::default());
        let info_matoken = mock_info("masomecoin");

        let mock_market = Market {
            ma_token_address: Addr::unchecked("masomecoin"),
            liquidity_index: Decimal::one(),
            liquidation_threshold: Decimal::from_ratio(5u128, 10u128),
            ..Default::default()
        };
        let market = th_init_market(deps.as_mut(), b"somecoin", &mock_market);
        let debt_mock_market = Market {
            borrow_index: Decimal::one(),
            ..Default::default()
        };
        let debt_market = th_init_market(deps.as_mut(), b"debtcoin", &debt_mock_market);

        deps.querier
            .set_oracle_price(b"somecoin".to_vec(), Decimal::from_ratio(1u128, 2u128));
        deps.querier
            .set_oracle_price(b"debtcoin".to_vec(), Decimal::from_ratio(2u128, 1u128));

        let sender_address = Addr::unchecked("fromaddr");
        let recipient_address = Addr::unchecked("toaddr");

        deps.querier.set_cw20_balances(
            Addr::unchecked("masomecoin"),
            &[(
                sender_address.clone(),
                Uint128::new(500_000) * SCALING_FACTOR,
            )],
        );

        {
            let mut sender_user = User::default();
            set_bit(&mut sender_user.collateral_assets, market.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &sender_address, &sender_user)
                .unwrap();
        }

        // Finalize transfer with sender not borrowing passes
        {
            let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
                sender_address: sender_address.clone(),
                recipient_address: recipient_address.clone(),
                sender_previous_balance: Uint128::new(1_000_000),
                recipient_previous_balance: Uint128::new(0),
                amount: Uint128::new(500_000),
            };

            let res = execute(deps.as_mut(), env.clone(), info_matoken.clone(), msg).unwrap();

            let sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
            let recipient_user = USERS.load(&deps.storage, &recipient_address).unwrap();
            assert!(get_bit(sender_user.collateral_assets, market.index).unwrap());
            // Should create user and set deposited to true as previous balance is 0
            assert!(get_bit(recipient_user.collateral_assets, market.index).unwrap());

            assert_eq!(
                res.events,
                vec![build_collateral_position_changed_event(
                    "somecoin",
                    true,
                    recipient_address.to_string()
                )]
            );
        }

        // Finalize transfer with health factor < 1 for sender doesn't go through
        {
            // set debt for user in order for health factor to be < 1
            let debt = Debt {
                amount_scaled: Uint128::new(500_000) * SCALING_FACTOR,
                uncollateralized: false,
            };
            let uncollateralized_debt = Debt {
                amount_scaled: Uint128::new(10_000) * SCALING_FACTOR,
                uncollateralized: true,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"debtcoin", &sender_address), &debt)
                .unwrap();
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (b"uncollateralized_debt", &sender_address),
                    &uncollateralized_debt,
                )
                .unwrap();
            let mut sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
            set_bit(&mut sender_user.borrowed_assets, debt_market.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &sender_address, &sender_user)
                .unwrap();
        }

        {
            let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
                sender_address: sender_address.clone(),
                recipient_address: recipient_address.clone(),
                sender_previous_balance: Uint128::new(1_000_000),
                recipient_previous_balance: Uint128::new(0),
                amount: Uint128::new(500_000),
            };

            let error_res =
                execute(deps.as_mut(), env.clone(), info_matoken.clone(), msg).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::CannotTransferTokenWhenInvalidHealthFactor {}
            );
        }

        // Finalize transfer with health factor > 1 for goes through
        {
            // set debt for user in order for health factor to be > 1
            let debt = Debt {
                amount_scaled: Uint128::new(1_000) * SCALING_FACTOR,
                uncollateralized: false,
            };
            let uncollateralized_debt = Debt {
                amount_scaled: Uint128::new(10_000u128) * SCALING_FACTOR,
                uncollateralized: true,
            };
            DEBTS
                .save(deps.as_mut().storage, (b"debtcoin", &sender_address), &debt)
                .unwrap();
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (b"uncollateralized_debt", &sender_address),
                    &uncollateralized_debt,
                )
                .unwrap();
            let mut sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
            set_bit(&mut sender_user.borrowed_assets, debt_market.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &sender_address, &sender_user)
                .unwrap();
        }

        {
            let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
                sender_address: sender_address.clone(),
                recipient_address: recipient_address.clone(),
                sender_previous_balance: Uint128::new(500_000),
                recipient_previous_balance: Uint128::new(500_000),
                amount: Uint128::new(500_000),
            };

            let res = execute(deps.as_mut(), env.clone(), info_matoken, msg).unwrap();

            let sender_user = USERS.load(&deps.storage, &sender_address).unwrap();
            let recipient_user = USERS.load(&deps.storage, &recipient_address).unwrap();
            // Should set deposited to false as: previous_balance - amount = 0
            assert!(!get_bit(sender_user.collateral_assets, market.index).unwrap());
            assert!(get_bit(recipient_user.collateral_assets, market.index).unwrap());

            assert_eq!(
                res.events,
                vec![build_collateral_position_changed_event(
                    "somecoin",
                    false,
                    sender_address.to_string()
                )]
            );
        }

        // Calling this with other token fails
        {
            let msg = ExecuteMsg::FinalizeLiquidityTokenTransfer {
                sender_address: sender_address,
                recipient_address: recipient_address,
                sender_previous_balance: Uint128::new(500_000),
                recipient_previous_balance: Uint128::new(500_000),
                amount: Uint128::new(500_000),
            };
            let info = mock_info("othertoken");

            let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(error_res, StdError::not_found("alloc::vec::Vec<u8>").into());
        }
    }

    #[test]
    fn test_uncollateralized_loan_limits() {
        let available_liquidity = 2000000000u128;
        let mut deps = th_setup(&[coin(available_liquidity, "somecoin")]);

        // Set tax data
        deps.querier.set_native_tax(
            Decimal::from_ratio(1u128, 100u128),
            &[(String::from("somecoin"), Uint128::new(100u128))],
        );

        let mock_market = Market {
            ma_token_address: Addr::unchecked("matoken"),
            borrow_index: Decimal::from_ratio(12u128, 10u128),
            liquidity_index: Decimal::from_ratio(8u128, 10u128),
            borrow_rate: Decimal::from_ratio(20u128, 100u128),
            liquidity_rate: Decimal::from_ratio(10u128, 100u128),
            reserve_factor: Decimal::from_ratio(1u128, 10u128),
            debt_total_scaled: Uint128::zero(),
            indexes_last_updated: 10000000,
            asset_type: AssetType::Native,
            ..Default::default()
        };

        // should get index 0
        let market_initial = th_init_market(deps.as_mut(), b"somecoin", &mock_market);

        let mut block_time = mock_market.indexes_last_updated + 10000u64;
        let initial_uncollateralized_loan_limit = Uint128::from(2400_u128);

        // Check that borrowers with uncollateralized debt cannot get an uncollateralized loan limit
        let existing_borrower_addr = Addr::unchecked("existing_borrower");

        let mut existing_borrower = User::default();
        set_bit(&mut existing_borrower.borrowed_assets, 0).unwrap();
        USERS
            .save(
                &mut deps.storage,
                &existing_borrower_addr,
                &existing_borrower,
            )
            .unwrap();

        let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            user_address: existing_borrower_addr.to_string(),
            new_limit: initial_uncollateralized_loan_limit,
        };
        let update_limit_env = mock_env_at_block_time(block_time);
        let info = mock_info("owner");
        let err = execute(
            deps.as_mut(),
            update_limit_env.clone(),
            info,
            update_limit_msg,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::UserHasCollateralizedDebt {});

        // Update uncollateralized loan limit for users without collateralized loans
        let borrower_addr = Addr::unchecked("borrower");

        let update_limit_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            user_address: borrower_addr.to_string(),
            new_limit: initial_uncollateralized_loan_limit,
        };

        // update limit as unauthorized user, should fail
        let info = mock_info("random");
        let error_res = execute(
            deps.as_mut(),
            update_limit_env.clone(),
            info,
            update_limit_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // Update borrower limit as owner
        let info = mock_info("owner");
        execute(deps.as_mut(), update_limit_env, info, update_limit_msg).unwrap();

        // check user's limit has been updated to the appropriate amount
        let limit = UNCOLLATERALIZED_LOAN_LIMITS
            .load(&deps.storage, (b"somecoin", &borrower_addr))
            .unwrap();
        assert_eq!(limit, initial_uncollateralized_loan_limit);

        // check user's uncollateralized debt flag is true (limit > 0)
        let debt = DEBTS
            .load(&deps.storage, (b"somecoin", &borrower_addr))
            .unwrap();
        assert!(debt.uncollateralized);

        // Borrow asset
        block_time += 1000_u64;
        let initial_borrow_amount =
            initial_uncollateralized_loan_limit.multiply_ratio(1_u64, 2_u64);
        let borrow_msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: initial_borrow_amount,
            recipient: None,
        };
        let borrow_env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        let res = execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

        let expected_params = th_get_expected_indices_and_rates(
            &market_initial,
            block_time,
            available_liquidity,
            TestUtilizationDeltas {
                less_liquidity: initial_borrow_amount.into(),
                more_debt: initial_borrow_amount.into(),
                ..Default::default()
            },
        );

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: borrower_addr.to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: String::from("somecoin"),
                        amount: initial_borrow_amount,
                    }
                )
                .unwrap()],
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "borrow"),
                attr("asset", "somecoin"),
                attr("user", "borrower"),
                attr("recipient", "borrower"),
                attr("amount", initial_borrow_amount.to_string()),
            ]
        );
        assert_eq!(
            res.events,
            vec![
                build_debt_position_changed_event("somecoin", true, "borrower".to_string()),
                th_build_interests_updated_event("somecoin", &expected_params)
            ]
        );

        // Check debt
        let user = USERS.load(&deps.storage, &borrower_addr).unwrap();
        assert!(get_bit(user.borrowed_assets, 0).unwrap());

        let debt = DEBTS
            .load(&deps.storage, (b"somecoin", &borrower_addr))
            .unwrap();

        let expected_debt_scaled_after_borrow = compute_scaled_amount(
            initial_borrow_amount,
            expected_params.borrow_index,
            ScalingOperation::Debt,
        )
        .unwrap();

        assert_eq!(expected_debt_scaled_after_borrow, debt.amount_scaled);

        // Borrow an amount less than initial limit but exceeding current limit
        let remaining_limit = initial_uncollateralized_loan_limit - initial_borrow_amount;
        let exceeding_limit = remaining_limit + Uint128::from(100_u64);

        block_time += 1000_u64;
        let borrow_msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: exceeding_limit,
            recipient: None,
        };
        let borrow_env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        let error_res = execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::BorrowAmountExceedsUncollateralizedLoanLimit {}
        );

        // Borrow a valid amount given uncollateralized loan limit
        block_time += 1000_u64;
        let borrow_msg = ExecuteMsg::Borrow {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            amount: remaining_limit,
            recipient: None,
        };
        let borrow_env = mock_env_at_block_time(block_time);
        let info = mock_info("borrower");
        execute(deps.as_mut(), borrow_env, info, borrow_msg).unwrap();

        // Set limit to zero
        let update_allowance_msg = ExecuteMsg::UpdateUncollateralizedLoanLimit {
            asset: Asset::Native {
                denom: "somecoin".to_string(),
            },
            user_address: borrower_addr.to_string(),
            new_limit: Uint128::zero(),
        };
        let allowance_env = mock_env_at_block_time(block_time);
        let info = mock_info("owner");
        execute(deps.as_mut(), allowance_env, info, update_allowance_msg).unwrap();

        // check user's allowance is zero
        let allowance = UNCOLLATERALIZED_LOAN_LIMITS
            .load(&deps.storage, (b"somecoin", &borrower_addr))
            .unwrap();
        assert_eq!(allowance, Uint128::zero());

        // check user's uncollateralized debt flag is false (limit == 0)
        let debt = DEBTS
            .load(&deps.storage, (b"somecoin", &borrower_addr))
            .unwrap();
        assert!(!debt.uncollateralized);
    }

    #[test]
    fn test_update_asset_collateral() {
        let mut deps = th_setup(&[]);

        let user_addr = Addr::unchecked(String::from("user"));

        let token_addr_1 = Addr::unchecked("depositedcoin1");
        let ma_token_addr_1 = Addr::unchecked("matoken1");
        let mock_market_1 = Market {
            ma_token_address: ma_token_addr_1.clone(),
            asset_type: AssetType::Cw20,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(40u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(60u128, 100u128),
            ..Default::default()
        };
        let token_addr_2 = Addr::unchecked("depositedcoin2");
        let ma_token_addr_2 = Addr::unchecked("matoken2");
        let mock_market_2 = Market {
            ma_token_address: ma_token_addr_2.clone(),
            asset_type: AssetType::Native,
            liquidity_index: Decimal::from_ratio(1u128, 2u128),
            borrow_index: Decimal::one(),
            max_loan_to_value: Decimal::from_ratio(50u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(80u128, 100u128),
            ..Default::default()
        };
        let token_addr_3 = Addr::unchecked("depositedcoin3");
        let ma_token_addr_3 = Addr::unchecked("matoken3");
        let mock_market_3 = Market {
            ma_token_address: ma_token_addr_3.clone(),
            asset_type: AssetType::Native,
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::from_ratio(2u128, 1u128),
            max_loan_to_value: Decimal::from_ratio(20u128, 100u128),
            liquidation_threshold: Decimal::from_ratio(40u128, 100u128),
            ..Default::default()
        };

        let market_1_initial =
            th_init_market(deps.as_mut(), token_addr_1.as_bytes(), &mock_market_1);
        let market_2_initial =
            th_init_market(deps.as_mut(), token_addr_2.as_bytes(), &mock_market_2);
        let market_3_initial =
            th_init_market(deps.as_mut(), token_addr_3.as_bytes(), &mock_market_3);

        // Set the querier to return exchange rates
        let token_1_exchange_rate = Decimal::from_ratio(2u128, 1u128);
        let token_2_exchange_rate = Decimal::from_ratio(3u128, 1u128);
        let token_3_exchange_rate = Decimal::from_ratio(4u128, 1u128);
        deps.querier
            .set_oracle_price(token_addr_1.as_bytes().to_vec(), token_1_exchange_rate);
        deps.querier
            .set_oracle_price(token_addr_2.as_bytes().to_vec(), token_2_exchange_rate);
        deps.querier
            .set_oracle_price(token_addr_3.as_bytes().to_vec(), token_3_exchange_rate);

        let env = mock_env(MockEnvParams::default());
        let info = mock_info(user_addr.as_str());

        {
            // Set second asset as collateral
            let mut user = User::default();
            set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &user_addr, &user)
                .unwrap();

            // Set the querier to return zero for the first asset
            deps.querier.set_cw20_balances(
                ma_token_addr_1.clone(),
                &[(user_addr.clone(), Uint128::zero())],
            );

            // Enable first market index which is currently disabled as collateral and ma-token balance is 0
            let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
                asset: Asset::Cw20 {
                    contract_addr: token_addr_1.to_string(),
                },
                enable: true,
            };
            let error_res =
                execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap_err();
            assert_eq!(
                error_res,
                ContractError::UserNoCollateralBalance {
                    user_address: user_addr.to_string(),
                    asset: String::from(token_addr_1.as_str())
                }
            );

            let user = USERS.load(&deps.storage, &user_addr).unwrap();
            let market_1_collateral =
                get_bit(user.collateral_assets, market_1_initial.index).unwrap();
            // Balance for first asset is zero so don't update bit
            assert!(!market_1_collateral);

            // Set the querier to return balance more than zero for the first asset
            deps.querier.set_cw20_balances(
                ma_token_addr_1.clone(),
                &[(user_addr.clone(), Uint128::new(100_000))],
            );

            // Enable first market index which is currently disabled as collateral and ma-token balance is more than 0
            let _res = execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
            let user = USERS.load(&deps.storage, &user_addr).unwrap();
            let market_1_collateral =
                get_bit(user.collateral_assets, market_1_initial.index).unwrap();
            // Balance for first asset is more than zero so update bit
            assert!(market_1_collateral);

            // Disable second market index
            let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
                asset: Asset::Native {
                    denom: token_addr_2.to_string(),
                },
                enable: false,
            };
            let _res = execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
            let user = USERS.load(&deps.storage, &user_addr).unwrap();
            let market_2_collateral =
                get_bit(user.collateral_assets, market_2_initial.index).unwrap();
            assert!(!market_2_collateral);
        }

        // User's health factor can't be less than 1 after disabling collateral
        {
            // Initialize user with market_1 and market_2 as collaterals
            // User borrows market_3
            let mut user = User::default();
            set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
            set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
            set_bit(&mut user.borrowed_assets, market_3_initial.index).unwrap();
            USERS
                .save(deps.as_mut().storage, &user_addr, &user)
                .unwrap();

            // Set the querier to return collateral balances (ma_token_1 and ma_token_2)
            let ma_token_1_balance_scaled = Uint128::new(150_000) * SCALING_FACTOR;
            deps.querier.set_cw20_balances(
                ma_token_addr_1.clone(),
                &[(user_addr.clone(), ma_token_1_balance_scaled.into())],
            );
            let ma_token_2_balance_scaled = Uint128::new(220_000) * SCALING_FACTOR;
            deps.querier.set_cw20_balances(
                ma_token_addr_2.clone(),
                &[(user_addr.clone(), ma_token_2_balance_scaled.into())],
            );

            // Calculate maximum debt for the user to have valid health factor
            let token_1_weighted_lt_in_uusd = compute_underlying_amount(
                ma_token_1_balance_scaled,
                get_updated_liquidity_index(&market_1_initial, env.block.time.seconds()).unwrap(),
            )
            .unwrap()
                * market_1_initial.liquidation_threshold
                * token_1_exchange_rate;
            let token_2_weighted_lt_in_uusd = compute_underlying_amount(
                ma_token_2_balance_scaled,
                get_updated_liquidity_index(&market_2_initial, env.block.time.seconds()).unwrap(),
            )
            .unwrap()
                * market_2_initial.liquidation_threshold
                * token_2_exchange_rate;
            let weighted_liquidation_threshold_in_uusd =
                token_1_weighted_lt_in_uusd + token_2_weighted_lt_in_uusd;
            let max_debt_for_valid_hf = Decimal::divide_uint128_by_decimal(
                weighted_liquidation_threshold_in_uusd,
                token_3_exchange_rate,
            )
            .unwrap();
            let token_3_debt_scaled = get_scaled_debt_amount(
                max_debt_for_valid_hf,
                &market_3_initial,
                env.block.time.seconds(),
            )
            .unwrap();

            // Set user to have max debt for valid health factor
            let debt = Debt {
                amount_scaled: token_3_debt_scaled,
                uncollateralized: false,
            };
            DEBTS
                .save(
                    deps.as_mut().storage,
                    (token_addr_3.as_bytes(), &user_addr),
                    &debt,
                )
                .unwrap();

            let user_position = get_user_position(
                deps.as_ref(),
                env.block.time.seconds(),
                &user_addr,
                Addr::unchecked("oracle"),
                &user,
                3,
            )
            .unwrap();
            // Should have valid health factor
            assert_eq!(
                user_position.health_status,
                UserHealthStatus::Borrowing(Decimal::one())
            );

            // Disable second market index
            let update_msg = ExecuteMsg::UpdateAssetCollateralStatus {
                asset: Asset::Native {
                    denom: token_addr_2.to_string(),
                },
                enable: false,
            };
            let res_error = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap_err();
            assert_eq!(
                res_error,
                ContractError::InvalidHealthFactorAfterDisablingCollateral {}
            )
        }
    }

    #[test]
    fn test_query_collateral() {
        let mut deps = th_setup(&[]);

        let user_addr = Addr::unchecked("user");

        // Setup first market containing a CW20 asset
        let cw20_contract_addr_1 = Addr::unchecked("depositedcoin1");
        deps.querier
            .set_cw20_symbol(cw20_contract_addr_1.clone(), "DP1".to_string());
        let market_1_initial = th_init_market(
            deps.as_mut(),
            cw20_contract_addr_1.as_bytes(),
            &Market {
                asset_type: AssetType::Cw20,
                ..Default::default()
            },
        );

        // Setup second market containing a native asset
        let market_2_initial = th_init_market(
            deps.as_mut(),
            String::from("uusd").as_bytes(),
            &Market {
                ..Default::default()
            },
        );

        // Set second market as collateral
        let mut user = User::default();
        set_bit(&mut user.collateral_assets, market_2_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &user_addr, &user)
            .unwrap();

        // Assert markets correctly return collateral status
        let res = query_user_collateral(deps.as_ref(), user_addr.clone()).unwrap();
        assert_eq!(res.collateral[0].denom, String::from("DP1"));
        assert!(!res.collateral[0].enabled);
        assert_eq!(res.collateral[1].denom, String::from("uusd"));
        assert!(res.collateral[1].enabled);

        // Set first market as collateral
        set_bit(&mut user.collateral_assets, market_1_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &user_addr, &user)
            .unwrap();

        // Assert markets correctly return collateral status
        let res = query_user_collateral(deps.as_ref(), user_addr).unwrap();
        assert_eq!(res.collateral[0].denom, String::from("DP1"));
        assert!(res.collateral[0].enabled);
        assert_eq!(res.collateral[1].denom, String::from("uusd"));
        assert!(res.collateral[1].enabled);
    }

    #[test]
    fn test_query_user_debt() {
        let mut deps = th_setup(&[]);

        let user_addr = Addr::unchecked("user");

        // Setup markets
        let cw20_contract_addr_1 = Addr::unchecked("cw20_coin_1");
        deps.querier
            .set_cw20_symbol(cw20_contract_addr_1.clone(), "CW20C1".to_string());
        let market_1_initial = th_init_market(
            deps.as_mut(),
            cw20_contract_addr_1.as_bytes(),
            &Market {
                asset_type: AssetType::Cw20,
                borrow_index: Decimal::one(),
                borrow_rate: Decimal::one(),
                ..Default::default()
            },
        );

        let _market_2_initial = th_init_market(
            deps.as_mut(),
            b"native_coin_1",
            &Market {
                borrow_index: Decimal::one(),
                borrow_rate: Decimal::one(),
                ..Default::default()
            },
        );

        let market_3_initial = th_init_market(
            deps.as_mut(),
            b"native_coin_2",
            &Market {
                borrow_index: Decimal::one(),
                borrow_rate: Decimal::one(),
                ..Default::default()
            },
        );

        // Set first and third market as borrowing assets
        let mut user = User::default();
        set_bit(&mut user.borrowed_assets, market_1_initial.index).unwrap();
        set_bit(&mut user.borrowed_assets, market_3_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &user_addr, &user)
            .unwrap();

        let env = mock_env(MockEnvParams::default());

        // Save debt for market 1
        let debt_amount_1 = Uint128::new(1234000u128);
        let debt_amount_scaled_1 =
            get_scaled_debt_amount(debt_amount_1, &market_1_initial, env.block.time.seconds())
                .unwrap();
        let debt_1 = Debt {
            amount_scaled: debt_amount_scaled_1,
            uncollateralized: false,
        };
        DEBTS
            .save(
                deps.as_mut().storage,
                (cw20_contract_addr_1.as_bytes(), &user_addr),
                &debt_1,
            )
            .unwrap();

        // Save debt for market 3
        let debt_amount_3 = Uint128::new(2221u128);
        let debt_amount_scaled_3 =
            get_scaled_debt_amount(debt_amount_3, &market_3_initial, env.block.time.seconds())
                .unwrap();
        let debt_3 = Debt {
            amount_scaled: debt_amount_scaled_3,
            uncollateralized: false,
        };
        DEBTS
            .save(
                deps.as_mut().storage,
                (b"native_coin_2", &user_addr),
                &debt_3,
            )
            .unwrap();

        let res = query_user_debt(deps.as_ref(), env, user_addr).unwrap();
        assert_eq!(
            res.debts[0],
            UserAssetDebtResponse {
                denom: "CW20C1".to_string(),
                asset_label: "cw20_coin_1".to_string(),
                asset_reference: cw20_contract_addr_1.as_bytes().to_vec(),
                asset_type: AssetType::Cw20,
                amount_scaled: debt_amount_scaled_1,
                amount: debt_amount_1
            }
        );
        assert_eq!(
            res.debts[1],
            UserAssetDebtResponse {
                denom: "native_coin_1".to_string(),
                asset_label: "native_coin_1".to_string(),
                asset_reference: b"native_coin_1".to_vec(),
                asset_type: AssetType::Native,
                amount_scaled: Uint128::zero(),
                amount: Uint128::zero()
            }
        );
        assert_eq!(
            res.debts[2],
            UserAssetDebtResponse {
                denom: "native_coin_2".to_string(),
                asset_label: "native_coin_2".to_string(),
                asset_reference: b"native_coin_2".to_vec(),
                asset_type: AssetType::Native,
                amount_scaled: debt_amount_scaled_3,
                amount: debt_amount_3
            }
        );
    }

    #[test]
    fn test_query_user_asset_debt() {
        let mut deps = th_setup(&[]);

        let user_addr = Addr::unchecked("user");

        // Setup markets
        let cw20_contract_addr_1 = Addr::unchecked("cw20_coin_1");
        deps.querier
            .set_cw20_symbol(cw20_contract_addr_1.clone(), "CW20C1".to_string());
        let market_1_initial = th_init_market(
            deps.as_mut(),
            cw20_contract_addr_1.as_bytes(),
            &Market {
                asset_type: AssetType::Cw20,
                borrow_index: Decimal::one(),
                borrow_rate: Decimal::from_ratio(1u128, 2u128),
                ..Default::default()
            },
        );

        let _market_2_initial = th_init_market(
            deps.as_mut(),
            b"native_coin_1",
            &Market {
                borrow_index: Decimal::one(),
                borrow_rate: Decimal::one(),
                ..Default::default()
            },
        );

        // Set first and third market as borrowing assets
        let mut user = User::default();
        set_bit(&mut user.borrowed_assets, market_1_initial.index).unwrap();
        USERS
            .save(deps.as_mut().storage, &user_addr, &user)
            .unwrap();

        let env = mock_env(MockEnvParams::default());

        // Save debt for market 1
        let debt_amount_1 = Uint128::new(1234567u128);
        let debt_amount_scaled_1 =
            get_scaled_debt_amount(debt_amount_1, &market_1_initial, env.block.time.seconds())
                .unwrap();
        let debt_1 = Debt {
            amount_scaled: debt_amount_scaled_1,
            uncollateralized: false,
        };
        DEBTS
            .save(
                deps.as_mut().storage,
                (cw20_contract_addr_1.as_bytes(), &user_addr),
                &debt_1,
            )
            .unwrap();

        // Check asset with existing debt
        {
            let res = query_user_asset_debt(
                deps.as_ref(),
                env.clone(),
                user_addr.clone(),
                Asset::Cw20 {
                    contract_addr: cw20_contract_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                res,
                UserAssetDebtResponse {
                    denom: "CW20C1".to_string(),
                    asset_label: "cw20_coin_1".to_string(),
                    asset_reference: cw20_contract_addr_1.as_bytes().to_vec(),
                    asset_type: AssetType::Cw20,
                    amount_scaled: debt_amount_scaled_1,
                    amount: debt_amount_1
                }
            );
        }

        // Check asset with no debt
        {
            let res = query_user_asset_debt(
                deps.as_ref(),
                env,
                user_addr,
                Asset::Native {
                    denom: "native_coin_1".to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                res,
                UserAssetDebtResponse {
                    denom: "native_coin_1".to_string(),
                    asset_label: "native_coin_1".to_string(),
                    asset_reference: b"native_coin_1".to_vec(),
                    asset_type: AssetType::Native,
                    amount_scaled: Uint128::zero(),
                    amount: Uint128::zero()
                }
            );
        }
    }

    // TEST HELPERS

    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(contract_balances);
        let env = mock_env(MockEnvParams::default());
        let info = mock_info("owner");
        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            ma_token_code_id: Some(1u64),
            close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
        };
        let msg = InstantiateMsg { config };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    fn th_init_market(deps: DepsMut, key: &[u8], market: &Market) -> Market {
        let mut index = 0;

        GLOBAL_STATE
            .update(
                deps.storage,
                |mut mm: GlobalState| -> StdResult<GlobalState> {
                    index = mm.market_count;
                    mm.market_count += 1;
                    Ok(mm)
                },
            )
            .unwrap();

        let new_market = Market {
            index,
            ..market.clone()
        };

        MARKETS.save(deps.storage, key, &new_market).unwrap();

        MARKET_REFERENCES_BY_INDEX
            .save(deps.storage, U32Key::new(index), &key.to_vec())
            .unwrap();

        MARKET_REFERENCES_BY_MA_TOKEN
            .save(deps.storage, &new_market.ma_token_address, &key.to_vec())
            .unwrap();

        new_market
    }

    #[derive(Default, Debug)]
    struct TestInterestResults {
        borrow_index: Decimal,
        liquidity_index: Decimal,
        borrow_rate: Decimal,
        liquidity_rate: Decimal,
        protocol_rewards_to_distribute: Uint128,
    }

    fn th_build_interests_updated_event(label: &str, ir: &TestInterestResults) -> Event {
        Event::new("interests_updated")
            .add_attribute("asset", label)
            .add_attribute("borrow_index", ir.borrow_index.to_string())
            .add_attribute("liquidity_index", ir.liquidity_index.to_string())
            .add_attribute("borrow_rate", ir.borrow_rate.to_string())
            .add_attribute("liquidity_rate", ir.liquidity_rate.to_string())
    }

    /// Deltas to be using in expected indices/rates results
    #[derive(Default, Debug)]
    struct TestUtilizationDeltas {
        less_liquidity: u128,
        more_debt: u128,
        less_debt: u128,
    }

    /// Takes a market before an action (ie: a borrow) among some test parameters
    /// used in that action and computes the expected indices and rates after that action.
    fn th_get_expected_indices_and_rates(
        market: &Market,
        block_time: u64,
        initial_liquidity: u128,
        deltas: TestUtilizationDeltas,
    ) -> TestInterestResults {
        let expected_indices = th_get_expected_indices(market, block_time);

        let expected_protocol_rewards_to_distribute =
            th_get_expected_protocol_rewards(market, &expected_indices);

        // When borrowing, new computed index is used for scaled amount
        let more_debt_scaled = compute_scaled_amount(
            Uint128::from(deltas.more_debt),
            expected_indices.borrow,
            ScalingOperation::Debt,
        )
        .unwrap();
        // When repaying, new computed index is used for scaled amount
        let less_debt_scaled = compute_scaled_amount(
            Uint128::from(deltas.less_debt),
            expected_indices.borrow,
            ScalingOperation::Debt,
        )
        .unwrap();
        // NOTE: Don't panic here so that the total repay of debt can be simulated
        // when less debt is greater than outstanding debt
        let new_debt_total_scaled =
            if (market.debt_total_scaled + more_debt_scaled) > less_debt_scaled {
                market.debt_total_scaled + more_debt_scaled - less_debt_scaled
            } else {
                Uint128::zero()
            };
        let dec_debt_total =
            compute_underlying_amount(new_debt_total_scaled, expected_indices.borrow).unwrap();
        let contract_current_balance = Uint128::from(initial_liquidity);
        let liquidity_taken = Uint128::from(deltas.less_liquidity);
        let dec_liquidity_total = contract_current_balance - liquidity_taken;
        let expected_utilization_rate =
            Decimal::from_ratio(dec_debt_total, dec_liquidity_total + dec_debt_total);

        // interest rates (make a copy and update those values to get the expeted irs)
        let mut market_copy = market.clone();
        update_market_interest_rates_with_model(
            &mock_env_at_block_time(block_time),
            &mut market_copy,
            expected_utilization_rate,
        )
        .unwrap();

        TestInterestResults {
            borrow_index: expected_indices.borrow,
            liquidity_index: expected_indices.liquidity,
            borrow_rate: market_copy.borrow_rate,
            liquidity_rate: market_copy.liquidity_rate,
            protocol_rewards_to_distribute: expected_protocol_rewards_to_distribute,
        }
    }

    /// Compute protocol income to be distributed (using values up to the instant
    /// before the contract call is made)
    fn th_get_expected_protocol_rewards(
        market: &Market,
        expected_indices: &TestExpectedIndices,
    ) -> Uint128 {
        let previous_borrow_index = market.borrow_index;
        let previous_debt_total =
            compute_underlying_amount(market.debt_total_scaled, previous_borrow_index).unwrap();
        let current_debt_total =
            compute_underlying_amount(market.debt_total_scaled, expected_indices.borrow).unwrap();
        let interest_accrued = if current_debt_total > previous_debt_total {
            current_debt_total - previous_debt_total
        } else {
            Uint128::zero()
        };
        interest_accrued * market.reserve_factor
    }

    /// Expected results for applying accumulated interest
    struct TestExpectedIndices {
        liquidity: Decimal,
        borrow: Decimal,
    }

    fn th_get_expected_indices(market: &Market, block_time: u64) -> TestExpectedIndices {
        let seconds_elapsed = block_time - market.indexes_last_updated;
        // market indices
        let expected_liquidity_index = calculate_applied_linear_interest_rate(
            market.liquidity_index,
            market.liquidity_rate,
            seconds_elapsed,
        )
        .unwrap();

        let expected_borrow_index = calculate_applied_linear_interest_rate(
            market.borrow_index,
            market.borrow_rate,
            seconds_elapsed,
        )
        .unwrap();

        TestExpectedIndices {
            liquidity: expected_liquidity_index,
            borrow: expected_borrow_index,
        }
    }
}
