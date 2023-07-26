use cosmwasm_std::{
    coin, to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, IbcMsg, IbcTimeout,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Item;
use mars_owner::{Owner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, AddressResponseItem, MarsAddressType},
    incentives, red_bank,
    rewards_collector::{Config, ConfigResponse, UpdateConfig},
};
use mars_utils::helpers::option_string_to_addr;

use crate::{
    helpers::{stringify_option_amount, unwrap_option_amount},
    ContractError, ContractResult,
};
pub struct Collector<'a> {
    /// Contract's owner
    pub owner: Owner<'a>,
    /// The contract's configurations
    pub config: Item<'a, Config>,
}

impl<'a> Default for Collector<'a> {
    fn default() -> Self {
        Self {
            owner: Owner::new("owner"),
            config: Item::new("config"),
        }
    }
}

impl<'a> Collector<'a> {
    fn update_owner(
        self,
        deps: DepsMut,
        info: MessageInfo,
        update: OwnerUpdate,
    ) -> ContractResult<Response> {
        Ok(self.owner.update(deps, info, update)?)
    }

    fn update_config(
        &self,
        deps: DepsMut,
        sender: Addr,
        new_cfg: UpdateConfig,
    ) -> ContractResult<Response> {
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
        } = new_cfg;

        cfg.address_provider =
            option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
        cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
        cfg.safety_fund_denom = safety_fund_denom.unwrap_or(cfg.safety_fund_denom);
        cfg.fee_collector_denom = fee_collector_denom.unwrap_or(cfg.fee_collector_denom);
        cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
        cfg.timeout_seconds = timeout_seconds.unwrap_or(cfg.timeout_seconds);
        cfg.slippage_tolerance = slippage_tolerance.unwrap_or(cfg.slippage_tolerance);

        cfg.validate()?;

        self.config.save(deps.storage, &cfg)?;

        Ok(Response::new().add_attribute("action", "mars/rewards-collector/update_config"))
    }

    fn withdraw_from_red_bank(
        &self,
        deps: DepsMut,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response> {
        let cfg = self.config.load(deps.storage)?;

        let red_bank_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &cfg.address_provider,
            MarsAddressType::RedBank,
        )?;

        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: red_bank_addr.to_string(),
            msg: to_binary(&red_bank::ExecuteMsg::Withdraw {
                denom: denom.clone(),
                amount,
                recipient: None,
                account_id: None,
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_message(withdraw_msg)
            .add_attribute("action", "withdraw_from_red_bank")
            .add_attribute("denom", denom)
            .add_attribute("amount", stringify_option_amount(amount)))
    }

    fn claim_incentive_rewards(
        &self,
        deps: DepsMut,
        start_after_collateral_denom: Option<String>,
        start_after_incentive_denom: Option<String>,
        limit: Option<u32>,
    ) -> ContractResult<Response> {
        let cfg = self.config.load(deps.storage)?;

        let incentives_addr = address_provider::helpers::query_contract_addr(
            deps.as_ref(),
            &cfg.address_provider,
            MarsAddressType::Incentives,
        )?;

        let claim_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: incentives_addr.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::ClaimRewards {
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

    fn swap_asset(
        &self,
        deps: DepsMut,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response> {
        let cfg = self.config.load(deps.storage)?;

        let swapper_addr = deps
            .querier
            .query_wasm_smart::<AddressResponseItem>(
                cfg.address_provider,
                &mars_red_bank_types::address_provider::QueryMsg::Address(MarsAddressType::Swapper),
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
                msg: to_binary(&mars_red_bank_types::swapper::ExecuteMsg::<Empty>::SwapExactIn {
                    coin_in: coin_in_safety_fund.clone(),
                    denom_out: cfg.safety_fund_denom,
                    slippage: cfg.slippage_tolerance,
                })?,
                funds: vec![coin_in_safety_fund],
            });
        }

        // execute the swap to fee collector denom, if the amount to swap is non-zero,
        // and if the denom is not already the fee collector denom
        if !amount_fee_collector.is_zero() && denom != cfg.fee_collector_denom {
            let coin_in_fee_collector = coin(amount_fee_collector.u128(), denom.clone());
            messages.push(WasmMsg::Execute {
                contract_addr: swapper_addr,
                msg: to_binary(&mars_red_bank_types::swapper::ExecuteMsg::<Empty>::SwapExactIn {
                    coin_in: coin_in_fee_collector.clone(),
                    denom_out: cfg.fee_collector_denom,
                    slippage: cfg.slippage_tolerance,
                })?,
                funds: vec![coin_in_fee_collector],
            });
        }

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "swap_asset")
            .add_attribute("denom", denom)
            .add_attribute("amount_safety_fund", amount_safety_fund)
            .add_attribute("amount_fee_collector", amount_fee_collector)
            .add_attribute("slippage_tolerance", cfg.slippage_tolerance.to_string()))
    }

    fn distribute_rewards(
        &self,
        deps: DepsMut,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response> {
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

        let transfer_msg = CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: cfg.channel_id,
            to_address: to_address.to_string(),
            amount: Coin {
                denom: denom.clone(),
                amount: amount_to_distribute,
            },
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(cfg.timeout_seconds)),
        });

        Ok(Response::new()
            .add_message(transfer_msg)
            .add_attribute("action", "distribute_rewards")
            .add_attribute("denom", denom)
            .add_attribute("amount", amount_to_distribute)
            .add_attribute("to", to_address))
    }

    fn query_config(&self, deps: Deps) -> StdResult<ConfigResponse> {
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
        })
    }
}

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{
        entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    };
    use mars_owner::OwnerInit::SetInitialOwner;
    use mars_red_bank_types::rewards_collector::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::Collector;
    use crate::ContractResult;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        let collector = Collector::default();
        let owner = msg.owner.clone();

        let cfg = Config::checked(deps.api, msg)?;
        cfg.validate()?;

        collector.owner.initialize(
            deps.storage,
            deps.api,
            SetInitialOwner {
                owner,
            },
        )?;

        collector.config.save(deps.storage, &cfg)?;

        Ok(Response::default())
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let collector = Collector::default();
        match msg {
            ExecuteMsg::UpdateOwner(update) => collector.update_owner(deps, info, update),
            ExecuteMsg::UpdateConfig {
                new_cfg,
            } => collector.update_config(deps, info.sender, new_cfg),
            ExecuteMsg::WithdrawFromRedBank {
                denom,
                amount,
            } => collector.withdraw_from_red_bank(deps, denom, amount),
            ExecuteMsg::DistributeRewards {
                denom,
                amount,
            } => collector.distribute_rewards(deps, env, denom, amount),
            ExecuteMsg::SwapAsset {
                denom,
                amount,
            } => collector.swap_asset(deps, env, denom, amount),
            ExecuteMsg::ClaimIncentiveRewards {
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            } => collector.claim_incentive_rewards(
                deps,
                start_after_collateral_denom,
                start_after_incentive_denom,
                limit,
            ),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let collector = Collector::default();
        match msg {
            QueryMsg::Config {} => to_binary(&collector.query_config(deps)?),
        }
    }
}
