use cosmwasm_std::{
    coin, to_json_binary, Addr, Binary, Coin, CosmosMsg, CustomMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Item;
use mars_owner::{Owner, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_types::{
    address_provider::{self, AddressResponseItem, MarsAddressType},
    credit_manager::{self, Action},
    incentives, red_bank,
    rewards_collector::{
        Config, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfig,
    },
    swapper::SwapperRoute,
};
use mars_utils::helpers::option_string_to_addr;

use crate::{
    helpers::{stringify_option_amount, unwrap_option_amount},
    ContractError, ContractResult, IbcTransferMsg,
};
pub struct Collector<'a, M: CustomMsg, I: IbcTransferMsg<M>> {
    /// Contract's owner
    pub owner: Owner<'a>,
    /// The contract's configurations
    pub config: Item<'a, Config>,
    /// Phantomdata for custom msg
    pub custom_msg: std::marker::PhantomData<M>,
    /// Phantomdata for IBC transfer msg
    pub ibc_transfer_msg: std::marker::PhantomData<I>,
}

impl<'a, M: CustomMsg, I: IbcTransferMsg<M>> Default for Collector<'a, M, I> {
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
    I: IbcTransferMsg<M>,
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
                amount,
            } => self.distribute_rewards(deps, env, denom, amount),
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
                denom,
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
            safety_fund_denom,
            fee_collector_denom,
            channel_id,
            timeout_seconds,
            slippage_tolerance,
            neutron_ibc_config,
        } = new_cfg;

        cfg.address_provider =
            option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
        cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
        cfg.safety_fund_denom = safety_fund_denom.unwrap_or(cfg.safety_fund_denom);
        cfg.fee_collector_denom = fee_collector_denom.unwrap_or(cfg.fee_collector_denom);
        cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
        cfg.timeout_seconds = timeout_seconds.unwrap_or(cfg.timeout_seconds);
        cfg.slippage_tolerance = slippage_tolerance.unwrap_or(cfg.slippage_tolerance);
        if neutron_ibc_config.is_some() {
            // override current config, otherwise leave previous one
            cfg.neutron_ibc_config = neutron_ibc_config;
        }

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

    pub fn swap_asset(
        &self,
        deps: DepsMut,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
        safety_fund_route: Option<SwapperRoute>,
        fee_collector_route: Option<SwapperRoute>,
        safety_fund_min_receive: Option<Uint128>,
        fee_collector_min_receive: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let swapper_addr = deps
            .querier
            .query_wasm_smart::<AddressResponseItem>(
                cfg.address_provider,
                &mars_types::address_provider::QueryMsg::Address(MarsAddressType::Swapper),
            )?
            .address;

        // if amount is None, swap the total balance
        let amount_to_swap =
            unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

        // split the amount to swap between the safety fund and the fee collector
        let amount_safety_fund = amount_to_swap * cfg.safety_tax_rate;
        let amount_fee_collector = amount_to_swap.checked_sub(amount_safety_fund)?;
        let mut messages = vec![];

        // execute the swap to safety fund denom, if the amount to swap is non-zero,
        // and if the denom is not already the safety fund denom
        if !amount_safety_fund.is_zero() && denom != cfg.safety_fund_denom {
            let coin_in_safety_fund = coin(amount_safety_fund.u128(), denom.clone());
            messages.push(WasmMsg::Execute {
                contract_addr: swapper_addr.clone(),
                msg: to_json_binary(
                    &mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                        coin_in: coin_in_safety_fund.clone(),
                        denom_out: cfg.safety_fund_denom,
                        min_receive: safety_fund_min_receive.ok_or(ContractError::InvalidMinReceive {reason: "required to pass 'safety_fund_min_receive' when swapped to safety fund denom".to_string()})?,
                        route: safety_fund_route,
                    },
                )?,
                funds: vec![coin_in_safety_fund],
            });
        }

        // execute the swap to fee collector denom, if the amount to swap is non-zero,
        // and if the denom is not already the fee collector denom
        if !amount_fee_collector.is_zero() && denom != cfg.fee_collector_denom {
            let coin_in_fee_collector = coin(amount_fee_collector.u128(), denom.clone());
            messages.push(WasmMsg::Execute {
                contract_addr: swapper_addr,
                msg: to_json_binary(
                    &mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                        coin_in: coin_in_fee_collector.clone(),
                        denom_out: cfg.fee_collector_denom,
                        min_receive: fee_collector_min_receive.ok_or(ContractError::InvalidMinReceive {reason: "required to pass 'fee_collector_min_receive' when swapped to fee collector denom".to_string()})?,
                        route: fee_collector_route,
                    },
                )?,
                funds: vec![coin_in_fee_collector],
            });
        }

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "swap_asset")
            .add_attribute("denom", denom)
            .add_attribute("amount_safety_fund", amount_safety_fund)
            .add_attribute("amount_fee_collector", amount_fee_collector))
    }

    pub fn distribute_rewards(
        &self,
        deps: DepsMut,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let to_address = if denom == cfg.safety_fund_denom {
            address_provider::helpers::query_module_addr(
                deps.as_ref(),
                &cfg.address_provider,
                MarsAddressType::SafetyFund,
            )?
        } else if denom == cfg.fee_collector_denom {
            address_provider::helpers::query_module_addr(
                deps.as_ref(),
                &cfg.address_provider,
                MarsAddressType::FeeCollector,
            )?
        } else {
            return Err(ContractError::AssetNotEnabledForDistribution {
                denom,
            });
        };

        let amount_to_distribute =
            unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

        let transfer_msg = I::ibc_transfer_msg(
            env,
            to_address.clone(),
            Coin {
                denom: denom.clone(),
                amount: amount_to_distribute,
            },
            cfg,
        )?;

        Ok(Response::new()
            .add_message(transfer_msg)
            .add_attribute("action", "distribute_rewards")
            .add_attribute("denom", denom)
            .add_attribute("amount", amount_to_distribute)
            .add_attribute("to", to_address))
    }

    pub fn query_config(&self, deps: Deps) -> StdResult<ConfigResponse> {
        let owner_state = self.owner.query(deps.storage)?;
        let cfg = self.config.load(deps.storage)?;
        Ok(ConfigResponse {
            owner: owner_state.owner,
            proposed_new_owner: owner_state.proposed,
            address_provider: cfg.address_provider.into(),
            safety_tax_rate: cfg.safety_tax_rate,
            safety_fund_denom: cfg.safety_fund_denom,
            fee_collector_denom: cfg.fee_collector_denom,
            channel_id: cfg.channel_id,
            timeout_seconds: cfg.timeout_seconds,
            slippage_tolerance: cfg.slippage_tolerance,
            neutron_ibc_config: cfg.neutron_ibc_config,
        })
    }
}
