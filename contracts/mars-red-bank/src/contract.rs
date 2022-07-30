use std::str;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use cw20_base::msg::InstantiateMarketingInfo;

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::asset::{build_send_asset_msg, get_asset_balance, Asset, AssetType};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::{
    cw20_get_balance, cw20_get_symbol, option_string_to_addr, zero_address,
};
use mars_outpost::red_bank::interest_rate_models::init_interest_rate_model;
use mars_outpost::red_bank::msg::{
    CreateOrUpdateConfig, ExecuteMsg, InitOrUpdateAssetParams, InstantiateMsg, QueryMsg, ReceiveMsg,
};
use mars_outpost::red_bank::{
    Config, ConfigResponse, Debt, GlobalState, Market, MarketInfo, MarketsListResponse, User,
    UserAssetCollateralResponse, UserAssetDebtResponse, UserCollateralResponse, UserDebtResponse,
    UserHealthStatus, UserPositionResponse,
};
use mars_outpost::{ma_token, math};

use crate::accounts::get_user_position;
use crate::error::ContractError;
use crate::events::{build_collateral_position_changed_event, build_debt_position_changed_event};
use crate::helpers::{
    get_asset_denom, get_asset_identifiers, get_bit, get_denom_amount_from_coins, set_bit,
    unset_bit,
};
use crate::interest_rates::{
    apply_accumulated_interests, get_scaled_debt_amount, get_scaled_liquidity_amount,
    get_underlying_debt_amount, get_underlying_liquidity_amount, update_interest_rates,
};
use crate::state::{
    CONFIG, DEBTS, GLOBAL_STATE, MARKETS, MARKET_REFERENCES_BY_INDEX,
    MARKET_REFERENCES_BY_MA_TOKEN, UNCOLLATERALIZED_LOAN_LIMITS, USERS,
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
        base_asset,
    } = msg.config;

    // All fields should be available
    let available = owner.is_some()
        && address_provider_address.is_some()
        && ma_token_code_id.is_some()
        && close_factor.is_some()
        && base_asset.is_some();

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
        base_asset: base_asset.unwrap(),
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

        ExecuteMsg::UpdateConfig {
            config,
        } => execute_update_config(deps, env, info, config),

        ExecuteMsg::InitAsset {
            asset,
            asset_params,
            asset_symbol,
        } => execute_init_asset(deps, env, info, asset, asset_params, asset_symbol),

        ExecuteMsg::InitAssetTokenCallback {
            reference,
        } => execute_init_asset_token_callback(deps, env, info, reference),

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

        ExecuteMsg::UpdateAssetCollateralStatus {
            asset,
            enable,
        } => execute_update_asset_collateral_status(deps, env, info, asset, enable),

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
        ReceiveMsg::DepositCw20 {
            on_behalf_of,
        } => {
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
        ReceiveMsg::RepayCw20 {
            on_behalf_of,
        } => {
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
        base_asset,
    } = new_config;

    // Update config
    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider_address =
        option_string_to_addr(deps.api, address_provider_address, config.address_provider_address)?;
    config.ma_token_code_id = ma_token_code_id.unwrap_or(config.ma_token_code_id);
    config.close_factor = close_factor.unwrap_or(config.close_factor);
    config.base_asset = base_asset.unwrap_or(config.base_asset);

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
            let new_market =
                create_market(env.block.time.seconds(), market_idx, asset_type, asset_params)?;

            // Save new market
            MARKETS.save(deps.storage, asset_reference.as_slice(), &new_market)?;

            // Save index to reference mapping
            MARKET_REFERENCES_BY_INDEX.save(deps.storage, market_idx, &asset_reference.to_vec())?;

            // Increment market count
            money_market.market_count += 1;
            GLOBAL_STATE.save(deps.storage, &money_market)?;

            let symbol = if let Some(asset_symbol) = asset_symbol_option {
                asset_symbol
            } else {
                match asset {
                    Asset::Native {
                        denom,
                    } => denom,
                    Asset::Cw20 {
                        contract_addr,
                    } => {
                        let contract_addr = deps.api.addr_validate(&contract_addr)?;
                        cw20_get_symbol(&deps.querier, contract_addr)?
                    }
                }
            };

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

/// Update uncollateralized loan limit by a given amount in base asset
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

    let mut user = USERS.may_load(deps.storage, &user_address)?.unwrap_or_default();

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
    response =
        update_interest_rates(&deps, &env, &mut market, Uint128::zero(), asset_label, response)?;
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
        return Err(ContractError::MarketNotActive {
            asset: asset_label,
        });
    }

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
            asset: asset_label,
        });
    }

    let withdraw_amount = match amount {
        Some(amount) => {
            // Check user has sufficient balance to send back
            if amount.is_zero() || amount > withdrawer_balance_before {
                return Err(ContractError::InvalidWithdrawAmount {
                    asset: asset_label,
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

    let mut response = Response::new();

    // if amount to withdraw equals the user's balance then unset collateral bit
    if asset_as_collateral && withdraw_amount == withdrawer_balance_before {
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
    response =
        update_interest_rates(&deps, &env, &mut market, withdraw_amount, &asset_label, response)?;
    MARKETS.save(deps.storage, asset_reference.as_slice(), &market)?;

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
    response = response.add_message(build_send_asset_msg(
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
        .add_attribute("burn_amount", burn_amount)
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
        return Err(ContractError::InvalidBorrowAmount {
            asset: asset_label,
        });
    }

    // Load market and user state
    let global_state = GLOBAL_STATE.load(deps.storage)?;
    let mut borrow_market = MARKETS.load(deps.storage, asset_reference.as_slice())?;

    if !borrow_market.active {
        return Err(ContractError::MarketNotActive {
            asset: asset_label,
        });
    }
    if !borrow_market.borrow_enabled {
        return Err(ContractError::BorrowNotEnabled {
            asset: asset_label,
        });
    }

    let uncollateralized_loan_limit = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (asset_reference.as_slice(), &borrower_address))?
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

    let addresses = address_provider::helpers::query_addresses(
        deps.as_ref(),
        &config.address_provider_address,
        vec![MarsContract::Oracle, MarsContract::ProtocolRewardsCollector],
    )?;
    let protocol_rewards_collector_address = &addresses[&MarsContract::ProtocolRewardsCollector];
    let oracle_address = &addresses[&MarsContract::Oracle];

    // Check if user can borrow specified amount
    let mut uncollateralized_debt = false;
    if uncollateralized_loan_limit.is_zero() {
        // Collateralized loan: check max ltv is not exceeded
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &borrower_address,
            oracle_address,
            &user,
            global_state.market_count,
        )?;

        let borrow_asset_price = if is_borrowing_asset {
            // if user was already borrowing, get price from user position
            user_position.get_asset_price(asset_reference.as_slice(), &asset_label)?
        } else {
            mars_outpost::oracle::helpers::query_price(
                deps.querier,
                oracle_address,
                asset_reference.clone(),
            )?
        };

        let borrow_amount_in_base_asset = borrow_amount * borrow_asset_price;

        let total_debt_in_base_asset_after_borrow =
            user_position.total_debt_in_base_asset.checked_add(borrow_amount_in_base_asset)?;
        if total_debt_in_base_asset_after_borrow > user_position.max_debt_in_base_asset {
            return Err(ContractError::BorrowAmountExceedsGivenCollateral {});
        }
    } else {
        // Uncollateralized loan: check borrow amount plus debt does not exceed uncollateralized loan limit
        uncollateralized_debt = true;

        let borrower_debt = DEBTS
            .may_load(deps.storage, (asset_reference.as_slice(), &borrower_address))?
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
        .may_load(deps.storage, (asset_reference.as_slice(), &borrower_address))?
        .unwrap_or(Debt {
            amount_scaled: Uint128::zero(),
            uncollateralized: uncollateralized_debt,
        });
    let borrow_amount_scaled =
        get_scaled_debt_amount(borrow_amount, &borrow_market, env.block.time.seconds())?;
    debt.amount_scaled = debt.amount_scaled.checked_add(borrow_amount_scaled)?;
    DEBTS.save(deps.storage, (asset_reference.as_slice(), &borrower_address), &debt)?;

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
    response = response.add_message(build_send_asset_msg(
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
        return Err(ContractError::MarketNotActive {
            asset: asset_label,
        });
    }

    // Cannot repay zero amount
    if repay_amount.is_zero() {
        return Err(ContractError::InvalidRepayAmount {
            asset: asset_label,
        });
    }

    // Check new debt
    let mut debt = DEBTS.load(deps.storage, (asset_reference, &user_address))?;

    if debt.amount_scaled.is_zero() {
        return Err(ContractError::CannotRepayZeroDebt {});
    }

    let config = CONFIG.load(deps.storage)?;

    let protocol_rewards_collector_address = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::ProtocolRewardsCollector,
    )?;

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
        let refund_msg = build_send_asset_msg(
            user_address.clone(),
            asset_label.clone(),
            asset_type,
            refund_amount,
        )?;
        response = response.add_message(refund_msg);
    } else {
        debt_amount_after = debt_amount_before - repay_amount;
    }

    let debt_amount_scaled_after =
        get_scaled_debt_amount(debt_amount_after, &market, env.block.time.seconds())?;
    debt.amount_scaled = debt_amount_scaled_after;
    DEBTS.save(deps.storage, (asset_reference, &user_address), &debt)?;

    let debt_amount_scaled_delta =
        debt_amount_scaled_before.checked_sub(debt_amount_scaled_after)?;

    market.debt_total_scaled = market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

    response =
        update_interest_rates(&deps, &env, &mut market, Uint128::zero(), &asset_label, response)?;
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
    if let Some(limit) = UNCOLLATERALIZED_LOAN_LIMITS
        .may_load(deps.storage, (debt_asset_reference.as_slice(), &user_address))?
    {
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
    let mut user_debt =
        DEBTS.load(deps.storage, (debt_asset_reference.as_slice(), &user_address))?;
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
    let oracle_address = &addresses[&MarsContract::Oracle];

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

    let collateral_and_debt_are_the_same_asset = debt_asset_reference == collateral_asset_reference;

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
    let collateral_price = user_position
        .get_asset_price(collateral_asset_reference.as_slice(), &collateral_asset_label)?;
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

    DEBTS.save(deps.storage, (debt_asset_reference.as_slice(), &user_address), &user_debt)?;

    let debt_market_debt_total_scaled_after =
        debt_market.debt_total_scaled.checked_sub(debt_amount_scaled_delta)?;

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

        MARKETS.save(deps.storage, asset_reference.as_slice(), &asset_market_after)?;
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

        MARKETS.save(deps.storage, debt_asset_reference.as_slice(), &debt_market_after)?;
    }

    // 7. Build response
    // refund sent amount in excess of actual debt amount to liquidate
    if refund_amount > Uint128::zero() {
        response = response.add_message(build_send_asset_msg(
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
    collateral_asset_label: &str,
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
            collateral_asset_label,
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
pub fn execute_update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    enable: bool,
) -> Result<Response, ContractError> {
    let user_address = info.sender;
    let mut user = USERS.may_load(deps.storage, &user_address)?.unwrap_or_default();

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
            deps.as_ref(),
            &config.address_provider_address,
            MarsContract::Oracle,
        )?;
        let user_position = get_user_position(
            deps.as_ref(),
            env.block.time.seconds(),
            &user_address,
            &oracle_address,
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

/// Update uncollateralized loan limit by a given amount in base asset
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
        deps.as_ref(),
        &config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position = get_user_position(
        deps.as_ref(),
        env.block.time.seconds(),
        &from_address,
        &oracle_address,
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
            let mut to_user = USERS.may_load(deps.storage, &to_address)?.unwrap_or_default();
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

        QueryMsg::Market {
            asset,
        } => to_binary(&query_market(deps, asset)?),

        QueryMsg::MarketsList {} => to_binary(&query_markets_list(deps)?),

        QueryMsg::UserDebt {
            user_address,
        } => {
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

        QueryMsg::UserCollateral {
            user_address,
        } => {
            let address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_user_collateral(deps, address)?)
        }

        QueryMsg::UncollateralizedLoanLimit {
            user_address,
            asset,
        } => {
            let user_address = deps.api.addr_validate(&user_address)?;
            to_binary(&query_uncollateralized_loan_limit(deps, user_address, asset)?)
        }

        QueryMsg::ScaledLiquidityAmount {
            asset,
            amount,
        } => to_binary(&query_scaled_liquidity_amount(deps, env, asset, amount)?),

        QueryMsg::ScaledDebtAmount {
            asset,
            amount,
        } => to_binary(&query_scaled_debt_amount(deps, env, asset, amount)?),

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
        } => to_binary(&query_underlying_debt_amount(deps, env, asset, amount_scaled)?),

        QueryMsg::UserPosition {
            user_address,
        } => {
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
            return Err(StdError::generic_err(format!("failed to load market for: {}", label)))
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
    let user = USERS.may_load(deps.storage, &user_address)?.unwrap_or_default();

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

    Ok(UserDebtResponse {
        debts: debts?,
    })
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
    let user = USERS.may_load(deps.storage, &address)?.unwrap_or_default();
    let oracle_address = address_provider::helpers::query_address(
        deps,
        &config.address_provider_address,
        MarsContract::Oracle,
    )?;
    let user_position = get_user_position(
        deps,
        env.block.time.seconds(),
        &address,
        &oracle_address,
        &user,
        global_state.market_count,
    )?;

    Ok(UserPositionResponse {
        total_collateral_in_base_asset: user_position.total_collateral_in_base_asset,
        total_debt_in_base_asset: user_position.total_debt_in_base_asset,
        total_collateralized_debt_in_base_asset: user_position
            .total_collateralized_debt_in_base_asset,
        max_debt_in_base_asset: user_position.max_debt_in_base_asset,
        weighted_liquidation_threshold_in_base_asset: user_position
            .weighted_liquidation_threshold_in_base_asset,
        health_status: user_position.health_status,
    })
}

// EVENTS
