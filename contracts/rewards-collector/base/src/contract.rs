use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, CustomMsg, CustomQuery, Deps, DepsMut, Env, IbcMsg,
    IbcTimeout, IbcTimeoutBlock, MessageInfo, Order, Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::{Bound, Item, Map};

use mars_outpost::address_provider::{self, MarsLocal, MarsRemote};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;
use mars_outpost::red_bank;
use mars_outpost::rewards_collector::{
    Config, CreateOrUpdateConfig, ExecuteMsg, InstantiateMsg, QueryMsg, RouteResponse,
    RoutesResponse,
};

use crate::helpers::{stringify_option_amount, unwrap_option_amount};
use crate::{ContractError, ContractResult, Route};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

pub struct CollectorBase<'a, R, M, Q>
where
    R: Route<M, Q>,
    M: CustomMsg,
    Q: CustomQuery,
{
    /// The contract's configurations
    pub config: Item<'a, Config<Addr>>,
    /// The trade route for each pair of input/output assets
    pub routes: Map<'a, (String, String), R>,
    /// Phantom data that holds the custom message type
    pub custom_msg: PhantomData<M>,
    /// Phantom data that holds the custom query type
    pub custom_query: PhantomData<Q>,
}

impl<'a, R, M, Q> Default for CollectorBase<'a, R, M, Q>
where
    R: Route<M, Q>,
    M: CustomMsg,
    Q: CustomQuery,
{
    fn default() -> Self {
        Self {
            config: Item::new("config"),
            routes: Map::new("routes"),
            custom_msg: PhantomData,
            custom_query: PhantomData,
        }
    }
}

impl<'a, R, M, Q> CollectorBase<'a, R, M, Q>
where
    R: Route<M, Q>,
    M: CustomMsg,
    Q: CustomQuery,
{
    pub fn instantiate(&self, deps: DepsMut<Q>, msg: InstantiateMsg) -> ContractResult<Response> {
        let cfg = msg.check(deps.api)?;
        cfg.validate()?;

        self.config.save(deps.storage, &cfg)?;

        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<R>,
    ) -> ContractResult<Response<M>> {
        match msg {
            ExecuteMsg::UpdateConfig {
                new_cfg,
            } => self.update_config(deps, info.sender, new_cfg),
            ExecuteMsg::SetRoute {
                denom_in,
                denom_out,
                route,
            } => self.set_route(deps, info.sender, denom_in, denom_out, route),
            ExecuteMsg::WithdrawFromRedBank {
                denom,
                amount,
            } => self.withdraw_from_red_bank(deps, denom, amount),
            ExecuteMsg::DistributeRewards {
                denom,
                amount,
            } => self.distribute_rewards(deps, env, denom, amount),
            ExecuteMsg::SwapAsset {
                denom,
                amount,
            } => self.swap_asset(deps, env, denom, amount),
        }
    }

    pub fn query(&self, deps: Deps<Q>, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Config {} => to_binary(&self.query_config(deps)?),
            QueryMsg::Route {
                denom_in,
                denom_out,
            } => to_binary(&self.query_route(deps, denom_in, denom_out)?),
            QueryMsg::Routes {
                start_after,
                limit,
            } => to_binary(&self.query_routes(deps, start_after, limit)?),
        }
    }

    fn update_config(
        &self,
        deps: DepsMut<Q>,
        sender: Addr,
        new_cfg: CreateOrUpdateConfig,
    ) -> ContractResult<Response<M>> {
        let mut cfg = self.config.load(deps.storage)?;

        if sender != cfg.owner {
            return Err(MarsError::Unauthorized {}.into());
        }

        let CreateOrUpdateConfig {
            owner,
            address_provider,
            safety_tax_rate,
            safety_fund_denom,
            fee_collector_denom,
            channel_id,
            timeout_revision,
            timeout_blocks,
            timeout_seconds,
            slippage_tolerance,
        } = new_cfg;

        cfg.owner = option_string_to_addr(deps.api, owner, cfg.owner)?;
        cfg.address_provider =
            option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
        cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
        cfg.safety_fund_denom = safety_fund_denom.unwrap_or(cfg.safety_fund_denom);
        cfg.fee_collector_denom = fee_collector_denom.unwrap_or(cfg.fee_collector_denom);
        cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
        cfg.timeout_revision = timeout_revision.unwrap_or(cfg.timeout_revision);
        cfg.timeout_blocks = timeout_blocks.unwrap_or(cfg.timeout_blocks);
        cfg.timeout_seconds = timeout_seconds.unwrap_or(cfg.timeout_seconds);
        cfg.slippage_tolerance = slippage_tolerance.unwrap_or(cfg.slippage_tolerance);

        cfg.validate()?;

        self.config.save(deps.storage, &cfg)?;

        Ok(Response::new().add_attribute("action", "mars/rewards-collector/update_config"))
    }

    fn set_route(
        &self,
        deps: DepsMut<Q>,
        sender: Addr,
        denom_in: String,
        denom_out: String,
        route: R,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        if sender != cfg.owner {
            return Err(MarsError::Unauthorized {}.into());
        }

        route.validate(&deps.querier, &denom_in, &denom_out)?;

        self.routes.save(deps.storage, (denom_in.clone(), denom_out.clone()), &route)?;

        Ok(Response::new()
            .add_attribute("action", "mars/rewards-collector/set_instructions")
            .add_attribute("denom_in", denom_in)
            .add_attribute("denom_out", denom_out)
            .add_attribute("route", route.to_string()))
    }

    fn withdraw_from_red_bank(
        &self,
        deps: DepsMut<Q>,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let red_bank_addr = address_provider::helpers::query_local_address(
            deps.as_ref(),
            &cfg.address_provider,
            MarsLocal::RedBank,
        )?;

        let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: red_bank_addr.to_string(),
            msg: to_binary(&red_bank::ExecuteMsg::Withdraw {
                denom: denom.clone(),
                amount,
                recipient: None,
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_message(withdraw_msg)
            .add_attribute("action", "outposts/rewards-collector/withdraw_from_red_bank")
            .add_attribute("denom", denom)
            .add_attribute("amount", stringify_option_amount(amount)))
    }

    fn swap_asset(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        // if amount is None, swap the total balance
        let amount_to_swap =
            unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

        // split the amount to swap between the safety fund and the fee collector
        let amount_safety_fund = amount_to_swap * cfg.safety_tax_rate;
        let amount_fee_collector = amount_to_swap.checked_sub(amount_safety_fund)?;
        let mut messages = vec![];

        if !amount_safety_fund.is_zero() {
            messages.push(
                self.routes
                    .load(deps.storage, (denom.clone(), cfg.safety_fund_denom))?
                    .build_swap_msg(
                        &env,
                        &deps.querier,
                        &denom,
                        amount_safety_fund,
                        cfg.slippage_tolerance,
                    )?,
            );
        }

        if !amount_fee_collector.is_zero() {
            messages.push(
                self.routes
                    .load(deps.storage, (denom.clone(), cfg.fee_collector_denom))?
                    .build_swap_msg(
                        &env,
                        &deps.querier,
                        &denom,
                        amount_fee_collector,
                        cfg.slippage_tolerance,
                    )?,
            );
        }

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "outposts/rewards-collector/swap_asset")
            .add_attribute("denom", denom)
            .add_attribute("amount_safety_fund", amount_safety_fund)
            .add_attribute("amount_fee_collector", amount_fee_collector)
            .add_attribute("slippage_tolerance", cfg.slippage_tolerance.to_string()))
    }

    fn distribute_rewards(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        denom: String,
        amount: Option<Uint128>,
    ) -> ContractResult<Response<M>> {
        let cfg = self.config.load(deps.storage)?;

        let to_address = if denom == cfg.safety_fund_denom {
            address_provider::helpers::query_remote_address(
                deps.as_ref(),
                &cfg.address_provider,
                MarsRemote::SafetyFund,
            )?
        } else if denom == cfg.fee_collector_denom {
            address_provider::helpers::query_remote_address(
                deps.as_ref(),
                &cfg.address_provider,
                MarsRemote::FeeCollector,
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
            to_address: to_address.clone(),
            amount: Coin {
                denom: denom.clone(),
                amount: amount_to_distribute,
            },
            timeout: IbcTimeout::with_both(
                IbcTimeoutBlock {
                    revision: cfg.timeout_revision,
                    height: env.block.height + cfg.timeout_blocks,
                },
                env.block.time.plus_seconds(cfg.timeout_seconds),
            ),
        });

        Ok(Response::new()
            .add_message(transfer_msg)
            .add_attribute("action", "outposts/rewards-collector/distribute_rewards")
            .add_attribute("denom", denom)
            .add_attribute("amount", amount_to_distribute)
            .add_attribute("to", to_address))
    }

    fn query_config(&self, deps: Deps<Q>) -> StdResult<Config<String>> {
        let cfg = self.config.load(deps.storage)?;
        Ok(cfg.into())
    }

    fn query_route(
        &self,
        deps: Deps<Q>,
        denom_in: String,
        denom_out: String,
    ) -> StdResult<RouteResponse<R>> {
        Ok(RouteResponse {
            denom_in: denom_in.clone(),
            denom_out: denom_out.clone(),
            route: self.routes.load(deps.storage, (denom_in, denom_out))?,
        })
    }

    fn query_routes(
        &self,
        deps: Deps<Q>,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> StdResult<RoutesResponse<R>> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(Bound::exclusive);

        self.routes
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (k, v) = item?;
                Ok(RouteResponse {
                    denom_in: k.0,
                    denom_out: k.1,
                    route: v,
                })
            })
            .collect()
    }
}
