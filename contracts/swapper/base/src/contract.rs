use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Deps,
    DepsMut, Env, MessageInfo, Order, Response, WasmMsg,
};
use cw_storage_plus::{Bound, Item, Map};

use rover::adapters::swap::{
    Config, EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, RouteResponse,
    RoutesResponse,
};
use rover::error::ContractError as RoverError;

use crate::{ContractResult, Route};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

pub struct SwapBase<'a, Q, M, R>
where
    Q: CustomQuery,
    M: CustomMsg,
    R: Route<M, Q>,
{
    /// The contract's config
    pub config: Item<'a, Config<Addr>>,
    /// The trade route for each pair of input/output assets
    pub routes: Map<'a, (String, String), R>,
    /// Phantom data holds generics
    pub custom_query: PhantomData<Q>,
    pub custom_message: PhantomData<M>,
}

impl<'a, Q, M, R> Default for SwapBase<'a, Q, M, R>
where
    Q: CustomQuery,
    M: CustomMsg,
    R: Route<M, Q>,
{
    fn default() -> Self {
        Self {
            config: Item::new("config"),
            routes: Map::new("routes"),
            custom_query: PhantomData,
            custom_message: PhantomData,
        }
    }
}

impl<'a, Q, M, R> SwapBase<'a, Q, M, R>
where
    Q: CustomQuery,
    M: CustomMsg,
    R: Route<M, Q>,
{
    pub fn instantiate(
        &self,
        deps: DepsMut<Q>,
        msg: InstantiateMsg,
    ) -> ContractResult<Response<M>> {
        self.config.save(
            deps.storage,
            &Config {
                owner: deps.api.addr_validate(&msg.owner)?,
            },
        )?;

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
            ExecuteMsg::UpdateConfig { owner } => self.update_config(deps, info.sender, owner),
            ExecuteMsg::SetRoute {
                denom_in,
                denom_out,
                route,
            } => self.set_route(deps, info.sender, denom_in, denom_out, route),
            ExecuteMsg::SwapExactIn {
                coin_in,
                denom_out,
                slippage,
            } => self.swap_exact_in(deps, env, info, coin_in, denom_out, slippage),
            ExecuteMsg::TransferResult {
                recipient,
                denom_in,
                denom_out,
            } => self.transfer_result(deps, env, info, recipient, denom_in, denom_out),
        }
    }

    pub fn query(&self, deps: Deps<Q>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
        let res = match msg {
            QueryMsg::Config {} => to_binary(&self.query_config(deps)?),
            QueryMsg::EstimateExactInSwap { coin_in, denom_out } => {
                to_binary(&self.estimate_exact_in_swap(deps, env, coin_in, denom_out)?)
            }
            QueryMsg::Route {
                denom_in,
                denom_out,
            } => to_binary(&self.query_route(deps, denom_in, denom_out)?),
            QueryMsg::Routes { start_after, limit } => {
                to_binary(&self.query_routes(deps, start_after, limit)?)
            }
        };
        res.map_err(Into::into)
    }

    fn query_config(&self, deps: Deps<Q>) -> ContractResult<Config<String>> {
        let cfg = self.config.load(deps.storage)?;
        Ok(Config {
            owner: cfg.owner.to_string(),
        })
    }

    fn query_route(
        &self,
        deps: Deps<Q>,
        denom_in: String,
        denom_out: String,
    ) -> ContractResult<RouteResponse<R>> {
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
    ) -> ContractResult<RoutesResponse<R>> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(Bound::exclusive);

        self.routes
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let ((denom_in, denom_out), route) = item?;
                Ok(RouteResponse {
                    denom_in,
                    denom_out,
                    route,
                })
            })
            .collect()
    }

    fn estimate_exact_in_swap(
        &self,
        deps: Deps<Q>,
        env: Env,
        coin_in: Coin,
        denom_out: String,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let route = self
            .routes
            .load(deps.storage, (coin_in.denom.clone(), denom_out))?;
        route.estimate_exact_in_swap(&deps.querier, &env, &coin_in)
    }

    fn swap_exact_in(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    ) -> ContractResult<Response<M>> {
        let swap_msg = self
            .routes
            .load(deps.storage, (coin_in.denom.clone(), denom_out.clone()))?
            .build_exact_in_swap_msg(&deps.querier, &env, &coin_in, slippage)?;

        // Check balance of result of swapper and send back result to sender
        let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::<R>::TransferResult {
                recipient: info.sender,
                denom_in: coin_in.denom.clone(),
                denom_out: denom_out.clone(),
            })?,
        });

        Ok(Response::new()
            .add_message(swap_msg)
            .add_message(transfer_msg)
            .add_attribute("action", "rover/swapper/swap_fn")
            .add_attribute("denom_in", coin_in.denom)
            .add_attribute("amount_in", coin_in.amount)
            .add_attribute("denom_out", denom_out)
            .add_attribute("slippage", slippage.to_string()))
    }

    fn transfer_result(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        recipient: Addr,
        denom_in: String,
        denom_out: String,
    ) -> ContractResult<Response<M>> {
        // Internal callback only
        if info.sender != env.contract.address {
            return Err(RoverError::Unauthorized {
                user: info.sender.to_string(),
                action: "transfer result".to_string(),
            }
            .into());
        };

        let denom_in_balance = deps
            .querier
            .query_balance(env.contract.address.clone(), denom_in)?;
        let denom_out_balance = deps
            .querier
            .query_balance(env.contract.address, denom_out)?;

        let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![denom_in_balance, denom_out_balance]
                .iter()
                .filter(|c| !c.amount.is_zero())
                .cloned()
                .collect(),
        });

        Ok(Response::new()
            .add_attribute("action", "rover/swapper/transfer_result")
            .add_message(transfer_msg))
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
            return Err(RoverError::Unauthorized {
                user: sender.to_string(),
                action: "set route".to_string(),
            }
            .into());
        };

        route.validate(&deps.querier, &denom_in, &denom_out)?;

        self.routes
            .save(deps.storage, (denom_in.clone(), denom_out.clone()), &route)?;

        Ok(Response::new()
            .add_attribute("action", "rover/base/set_route")
            .add_attribute("denom_in", denom_in)
            .add_attribute("denom_out", denom_out)
            .add_attribute("route", route.to_string()))
    }

    fn update_config(
        &self,
        deps: DepsMut<Q>,
        sender: Addr,
        owner: Option<String>,
    ) -> ContractResult<Response<M>> {
        let mut cfg = self.config.load(deps.storage)?;
        if sender != cfg.owner {
            return Err(RoverError::Unauthorized {
                user: sender.to_string(),
                action: "update owner".to_string(),
            }
            .into());
        };

        let mut response =
            Response::new().add_attribute("action", "rover/swapper-base/update_config");

        if let Some(addr_str) = owner {
            cfg.owner = deps.api.addr_validate(&addr_str)?;
            response = response
                .add_attribute("key", "owner")
                .add_attribute("value", addr_str);
        }

        self.config.save(deps.storage, &cfg)?;

        Ok(response)
    }
}
