use cosmwasm_std::{
    coin, to_json_binary, Addr, Binary, Coin, CosmosMsg, CustomMsg, Decimal, Deps, DepsMut, Empty,
    Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Item;
use mars_owner::{Owner, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_types::{
    address_provider::{self, AddressResponseItem, MarsAddressType},
    credit_manager::{self, Action},
    incentives,
    oracle::ActionKind,
    red_bank,
    rewards_collector::{
        Config, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfig,
    },
    swapper::SwapperRoute,
};
use mars_utils::helpers::option_string_to_addr;

use crate::{
    helpers::{stringify_option_amount, unwrap_option_amount},
    ContractError, ContractResult, TransferMsg,
};

pub struct Collector<'a, M: CustomMsg, I: TransferMsg<M>> {
    /// Contract's owner
    pub owner: Owner<'a>,
    /// The contract's configurations
    pub config: Item<'a, Config>,
    /// Phantomdata for custom msg
    pub custom_msg: std::marker::PhantomData<M>,
    /// Phantomdata for IBC transfer msg
    pub ibc_transfer_msg: std::marker::PhantomData<I>,
}

impl<'a, M: CustomMsg, I: TransferMsg<M>> Default for Collector<'a, M, I> {
    fn default() -> Self {
        Self {
            owner: Owner::new("owner"),
            config: Item::new("config"),
            custom_msg: std::marker::PhantomData,
            ibc_transfer_msg: std::marker::PhantomData,
        }
    }
}

impl<'a, M, I> Collector<'a, M, I>
where
    M: CustomMsg,
    I: TransferMsg<M>,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        let owner = msg.owner.clone();

        let cfg = Config::checked(deps.api, msg)?;
        cfg.validate()?;

        self.owner.initialize(
            deps.storage,
            deps.api,
            SetInitialOwner {
                owner,
            },
        )?;

        self.config.save(deps.storage, &cfg)?;

        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response<M>> {
        match msg {
            ExecuteMsg::UpdateOwner(update) => self.update_owner(deps, info, update),
            ExecuteMsg::UpdateConfig {
                new_cfg,
            } => self.update_config(deps, info.sender, new_cfg),
            ExecuteMsg::WithdrawFromRedBank {
                denom,
                amount,
            } => self.withdraw_from_red_bank(deps, denom, amount),
            ExecuteMsg::WithdrawFromCreditManager {
                account_id,
                actions,
            } => self.withdraw_from_credit_manager(deps, account_id, actions),
            ExecuteMsg::DistributeRewards {
                denom,
            } => self.distribute_rewards(deps, &env, &denom),
            ExecuteMsg::SwapAsset {
                denom,
                amount,
                safety_fund_route,
                fee_collector_route,
                safety_fund_min_receive,
                fee_collector_min_receive,
            } => self.swap_asset(
                deps,
                env,
                &denom,
                amount,
                safety_fund_route,
                fee_collector_route,
                safety_fund_min_receive,
                fee_collector_min_receive,
            ),
            ExecuteMsg::ClaimIncentiveRewards {
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            } => self.claim_incentive_rewards(
                deps,
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            ),
        }
    }

    pub fn query(&self, deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Config {} => to_json_binary(&self.query_config(deps)?),
        }
    }

    pub fn update_owner(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        update: OwnerUpdate,
    ) -> ContractResult<Response<M>> {
        Ok(self.owner.update(deps, info, update)?)
    }

    pub fn update_config(
        &self,
        deps: DepsMut,
        sender: Addr,
        new_cfg: UpdateConfig,
    ) -> ContractResult<Response<M>> {
        self.owner.assert_owner(deps.storage, &sender)?;

        let mut cfg = self.config.load(deps.storage)?;

        let UpdateConfig {
            address_provider,
            safety_tax_rate,
            revenue_share_tax_rate,
            safety_fund_config,
            revenue_share_config,
            fee_collector_config,
            channel_id,
            timeout_seconds,
            slippage_tolerance,
        } = new_cfg;

        cfg.address_provider =
            option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
        cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
        cfg.revenue_share_tax_rate = revenue_share_tax_rate.unwrap_or(cfg.revenue_share_tax_rate);
        cfg.safety_fund_config = safety_fund_config.unwrap_or(cfg.safety_fund_config);
        cfg.revenue_share_config = revenue_share_config.unwrap_or(cfg.revenue_share_config);
        cfg.fee_collector_config = fee_collector_config.unwrap_or(cfg.fee_collector_config);
        cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
        cfg.timeout_seconds = timeout_seconds.unwrap_or(cfg.timeout_seconds);
        cfg.slippage_tolerance = slippage_tolerance.unwrap_or(cfg.slippage_tolerance);

        cfg.validate()?;

        self.config.save(deps.storage, &cfg)?;

        Ok(Response::new().add_attribute("action", "mars/rewards-collector/update_config"))
    }

    pub fn withdraw_from_red_bank(
        &self,
        deps: DepsMut,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let red_bank_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &cfg.address_provider,
            MarsAddressType::RedBank,
        )?;

        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: red_bank_addr.to_string(),
            msg: to_json_binary(&red_bank::ExecuteMsg::Withdraw {
                denom: denom.clone(),
                amount,
                recipient: None,
                account_id: None,
                liquidation_related: None,
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_message(withdraw_msg)
            .add_attribute("action", "withdraw_from_red_bank")
            .add_attribute("denom", denom)
            .add_attribute("amount", stringify_option_amount(amount)))
    }

    pub fn withdraw_from_credit_manager(
        &self,
        deps: DepsMut,
        account_id: String,
        actions: Vec<Action>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let valid_actions = actions.iter().all(|action| {
            matches!(action, Action::Withdraw(..) | Action::WithdrawLiquidity { .. })
        });
        if !valid_actions {
            return Err(ContractError::InvalidActionsForCreditManager {});
        }

        let cm_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &cfg.address_provider,
            MarsAddressType::CreditManager,
        )?;

        let withdraw_from_cm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cm_addr.to_string(),
            msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
                account_id: Some(account_id.clone()),
                account_kind: None,
                actions,
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_message(withdraw_from_cm_msg)
            .add_attribute("action", "withdraw_from_credit_manager")
            .add_attribute("account_id", account_id))
    }

    pub fn claim_incentive_rewards(
        &self,
        deps: DepsMut,
        start_after_collateral_denom: Option<String>,
        start_after_incentive_denom: Option<String>,
        limit: Option<u32>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let incentives_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &cfg.address_provider,
            MarsAddressType::Incentives,
        )?;

        let claim_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: incentives_addr.to_string(),
            msg: to_json_binary(&incentives::ExecuteMsg::ClaimRewards {
                account_id: None,
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_message(claim_msg)
            .add_attribute("action", "claim_incentive_rewards"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn swap_asset(
        &self,
        deps: DepsMut,
        env: Env,
        denom: &str,
        amount: Option<Uint128>,
        safety_fund_route: Option<SwapperRoute>,
        fee_collector_route: Option<SwapperRoute>,
        safety_fund_min_receive: Option<Uint128>,
        fee_collector_min_receive: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        // if amount is None, swap the total balance
        let amount_to_swap =
            unwrap_option_amount(&deps.querier, &env.contract.address, denom, amount)?;

        // split the amount to swap between the safety fund, fee collector and the revenue share
        // we combine revenue fund and safety fund because they are the same denom
        let rf_and_sf_combined = amount_to_swap
            .checked_mul_floor(cfg.safety_tax_rate.checked_add(cfg.revenue_share_tax_rate)?)?;
        let fc_amount = amount_to_swap.checked_sub(rf_and_sf_combined)?;

        let mut messages = vec![];
        let addresses = &deps.querier.query_wasm_smart::<Vec<AddressResponseItem>>(
            cfg.address_provider,
            &address_provider::QueryMsg::Addresses(vec![
                MarsAddressType::Swapper,
                MarsAddressType::Oracle,
            ]),
        )?;

        let swapper_addr = &addresses[0].address;
        let oracle_addr = &addresses[1].address;

        let asset_in_price = deps
            .querier
            .query_wasm_smart::<mars_types::oracle::PriceResponse>(
                oracle_addr.to_string(),
                &mars_types::oracle::QueryMsg::Price {
                    denom: denom.to_string(),
                    kind: Some(ActionKind::Default),
                },
            )?
            .price;

        // apply slippage to asset in price. Creating this variable means we only need to apply
        // slippage tolerance calculation once, instead of for each denom
        let slippage_adjusted_asset_in_price =
            asset_in_price.checked_mul(Decimal::one().checked_sub(cfg.slippage_tolerance)?)?;

        // execute the swap to safety fund denom, if the amount to swap is non-zero,
        // and if the denom is not already the safety fund denom
        // Note that revenue share is included in this swap as they are the same denom
        if !rf_and_sf_combined.is_zero() && denom != cfg.safety_fund_config.target_denom {
            let swap_msg = self.swap_asset_to_reward(
                &deps,
                oracle_addr,
                &denom.to_string(),
                rf_and_sf_combined,
                slippage_adjusted_asset_in_price,
                safety_fund_min_receive.ok_or(
                    ContractError::InvalidMinReceive {
                        reason: "required to pass 'safety_fund_min_receive' when swapping safety fund amount".to_string()
                    }
                )?,
                &cfg.safety_fund_config.target_denom,
                safety_fund_route,
                swapper_addr,
            )?;

            messages.push(swap_msg);
        }

        // execute the swap to fee collector denom, if the amount to swap is non-zero,
        // and if the denom is not already the fee collector denom
        if !fc_amount.is_zero() && denom != cfg.fee_collector_config.target_denom {
            let swap_msg = self.swap_asset_to_reward(
                &deps,
                oracle_addr,
                &denom.to_string(),
                fc_amount,
                slippage_adjusted_asset_in_price,
                fee_collector_min_receive.ok_or(
                    ContractError::InvalidMinReceive {
                        reason: "required to pass 'fee_collector_min_receive' when swapping to fee collector".to_string()
                    }
                )?,
                &cfg.fee_collector_config.target_denom,
                fee_collector_route,
                swapper_addr,
            )?;

            messages.push(swap_msg);
        }

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "swap_asset")
            .add_attribute("denom", denom)
            .add_attribute("amount_safety_fund", rf_and_sf_combined)
            .add_attribute("amount_fee_collector", fc_amount))
    }

    fn swap_asset_to_reward(
        &self,
        deps: &DepsMut,
        oracle_addr: &str,
        asset_in_denom: &String,
        asset_in_amount: Uint128,
        slippage_adjusted_asset_in_price: Decimal,
        min_receive: Uint128,
        target_reward_denom: &String,
        target_route: Option<SwapperRoute>,
        swapper_addr: &str,
    ) -> Result<WasmMsg, ContractError> {
        let target_fund_price = deps
            .querier
            .query_wasm_smart::<mars_types::oracle::PriceResponse>(
                oracle_addr.to_string(),
                &mars_types::oracle::QueryMsg::Price {
                    denom: target_reward_denom.to_string(),
                    kind: Some(ActionKind::Default),
                },
            )?
            .price;

        self.ensure_min_receive_within_slippage_tolerance(
            asset_in_denom.to_string(),
            target_reward_denom.to_string(),
            asset_in_amount,
            slippage_adjusted_asset_in_price,
            target_fund_price,
            min_receive,
        )?;

        self.generate_swap_msg(
            swapper_addr,
            asset_in_denom,
            asset_in_amount,
            target_reward_denom,
            min_receive,
            target_route,
        )
    }

    fn generate_swap_msg(
        &self,
        swapper_addr: &str,
        denom_in: &str,
        amount_in: Uint128,
        denom_out: &str,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    ) -> Result<WasmMsg, ContractError> {
        Ok(WasmMsg::Execute {
            contract_addr: swapper_addr.to_string(),
            msg: to_json_binary(&mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                coin_in: coin(amount_in.u128(), denom_in),
                denom_out: denom_out.to_string(),
                min_receive,
                route,
            })?,
            funds: vec![coin(amount_in.u128(), denom_in)],
        })
    }

    /// Ensure the slippage is not greater than what is tolerated in contract config
    /// We do this by calculating the minimum_price and applying that to the min receive
    /// Calculation is as follows:
    /// Safety_denom price in oracle is 2
    /// slippage_adjusted_asset_in_price (calculated via oracle) is 9.5
    /// pair price = 9.5 / 2 = 4.75
    /// minimum_tolerated = 4.75 * amount_in
    fn ensure_min_receive_within_slippage_tolerance(
        &self,
        asset_in_denom: String,
        asset_out_denom: String,
        amount_in: Uint128,
        slippage_adjusted_asset_in_price: Decimal,
        asset_out_price: Decimal,
        min_receive: Uint128,
    ) -> Result<(), ContractError> {
        // The price of the asset to be swapped, denominated in the output asset denom
        let asset_out_denominated_price =
            slippage_adjusted_asset_in_price.checked_div(asset_out_price)?;
        let min_receive_lower_limit = amount_in.checked_mul_floor(asset_out_denominated_price)?;

        if min_receive_lower_limit > min_receive {
            return Err(ContractError::SlippageLimitExceeded {
                denom_in: asset_in_denom,
                denom_out: asset_out_denom,
                min_receive_minimum: min_receive_lower_limit,
                min_receive_given: min_receive,
            });
        }

        Ok(())
    }

    pub fn distribute_rewards(
        &self,
        deps: DepsMut,
        env: &Env,
        denom: &str,
    ) -> ContractResult<Response<M>> {
        let mut res = Response::new().add_attribute("action", "distribute_rewards");
        let mut msgs: Vec<CosmosMsg<M>> = vec![];

        // Configs
        let cfg = &self.config.load(deps.storage)?;
        let safety_fund_config = &cfg.safety_fund_config;
        let revenue_share_config = &cfg.revenue_share_config;
        let fee_collector_config = &cfg.fee_collector_config;

        // Get specified denom balance
        let balance = deps.querier.query_balance(env.contract.address.as_str(), denom)?;
        if balance.amount == Uint128::zero() {
            return Ok(res.add_attribute("denom", denom).add_attribute("amount", "zero"));
        }

        if denom == safety_fund_config.target_denom {
            // When distributing to the safety fund we need to split by safety fund and revenue share,
            // as we enforce that they have the same denom in the configuration
            let sf_proportion = if cfg.revenue_share_tax_rate.is_zero() {
                Decimal::one()
            } else {
                cfg.safety_tax_rate
                    .checked_div(cfg.safety_tax_rate.checked_add(cfg.revenue_share_tax_rate)?)?
            };

            // Amounts to send
            let sf_amount = balance.amount.checked_mul_floor(sf_proportion)?;
            let rs_amount = balance.amount.checked_sub(sf_amount)?;

            // Fetch our target addresses for distribution
            let contracts = vec![MarsAddressType::SafetyFund, MarsAddressType::RevenueShare];
            let addresses = address_provider::helpers::query_contract_addrs(
                deps.as_ref(),
                &cfg.address_provider,
                contracts,
            )?;
            let sf_address = &addresses[&MarsAddressType::SafetyFund];
            let rs_address = &addresses[&MarsAddressType::RevenueShare];

            // Generate distribute msg
            let sf_distribute_msg = I::transfer_msg(
                env,
                sf_address.as_str(),
                Coin {
                    denom: denom.to_string(),
                    amount: sf_amount,
                },
                cfg,
                &safety_fund_config.transfer_type,
            )?;
            msgs.push(sf_distribute_msg);

            res = res
                .add_attribute("address_type", MarsAddressType::SafetyFund.to_string())
                .add_attribute("to", sf_address)
                .add_attribute("amount", sf_amount);

            // if the revenue share amount is non-zero, we need to send that portion also
            if !rs_amount.is_zero() {
                let revenue_share_distribute_msg = I::transfer_msg(
                    env,
                    rs_address.as_str(),
                    Coin {
                        denom: denom.to_string(),
                        amount: rs_amount,
                    },
                    cfg,
                    &revenue_share_config.transfer_type,
                )?;

                msgs.push(revenue_share_distribute_msg);
                res = res
                    .add_attribute("address_type", MarsAddressType::RevenueShare.to_string())
                    .add_attribute("to", rs_address)
                    .add_attribute("amount", rs_amount);
            }
        } else if denom == fee_collector_config.target_denom {
            let fee_collector_address = address_provider::helpers::query_contract_addr(
                deps.as_ref(),
                &cfg.address_provider,
                MarsAddressType::FeeCollector,
            )?;
            let fee_collector_distribute_msg = I::transfer_msg(
                env,
                fee_collector_address.as_str(),
                Coin {
                    denom: denom.to_string(),
                    amount: balance.amount,
                },
                cfg,
                &fee_collector_config.transfer_type,
            )?;

            msgs.push(fee_collector_distribute_msg);

            res = res
                .add_attribute("address_type", MarsAddressType::FeeCollector.to_string())
                .add_attribute("to", fee_collector_address)
                .add_attribute("amount", balance.amount);
        } else {
            return Err(ContractError::AssetNotEnabledForDistribution {
                denom: denom.to_string(),
            });
        }

        Ok(res.add_messages(msgs))
    }

    pub fn query_config(&self, deps: Deps) -> StdResult<ConfigResponse> {
        let owner_state = self.owner.query(deps.storage)?;
        let cfg = self.config.load(deps.storage)?;
        Ok(ConfigResponse {
            owner: owner_state.owner,
            proposed_new_owner: owner_state.proposed,
            address_provider: cfg.address_provider.into(),
            safety_tax_rate: cfg.safety_tax_rate,
            revenue_share_tax_rate: cfg.revenue_share_tax_rate,
            safety_fund_config: cfg.safety_fund_config,
            revenue_share_config: cfg.revenue_share_config,
            fee_collector_config: cfg.fee_collector_config,
            channel_id: cfg.channel_id,
            timeout_seconds: cfg.timeout_seconds,
            slippage_tolerance: cfg.slippage_tolerance,
        })
    }
}
