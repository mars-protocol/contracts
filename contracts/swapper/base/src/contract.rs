use std::marker::PhantomData;

use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Deps,
    DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw_paginate::paginate_map;
use cw_storage_plus::{Bound, Item, Map};
use mars_owner::{Owner, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, RouteResponse,
    RoutesResponse, SwapperRoute,
};

use crate::{Config, ContractError, ContractResult, Route};

// Max allowed slippage percentage for swap
const MAX_SLIPPAGE_PERCENTAGE: u64 = 10;

pub struct SwapBase<'a, Q, M, R, C>
where
    Q: CustomQuery,
    M: CustomMsg,
    C: Config,
    R: Route<M, Q, C>,
{
    /// The contract's owner who has special rights to update contract
    pub owner: Owner<'a>,
    /// The trade route for each pair of input/output assets
    pub routes: Map<'a, (String, String), R>,
    /// Custom config
    pub config: Item<'a, C>,
    /// Phantom data holds generics
    pub custom_query: PhantomData<Q>,
    pub custom_message: PhantomData<M>,
}

impl<'a, Q, M, R, C> Default for SwapBase<'a, Q, M, R, C>
where
    Q: CustomQuery,
    M: CustomMsg,
    C: Config,
    R: Route<M, Q, C>,
{
    fn default() -> Self {
        Self {
            owner: Owner::new("owner"),
            routes: Map::new("routes"),
            config: Item::new("config"),
            custom_query: PhantomData,
            custom_message: PhantomData,
        }
    }
}

impl<'a, Q, M, R, C> SwapBase<'a, Q, M, R, C>
where
    Q: CustomQuery,
    M: CustomMsg,
    C: Config,
    R: Route<M, Q, C>,
{
    pub fn instantiate(
        &self,
        deps: DepsMut<Q>,
        msg: InstantiateMsg,
    ) -> ContractResult<Response<M>> {
        self.owner.initialize(
            deps.storage,
            deps.api,
            SetInitialOwner {
                owner: msg.owner,
            },
        )?;
        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<R, C>,
    ) -> ContractResult<Response<M>> {
        match msg {
            ExecuteMsg::UpdateOwner(update) => self.update_owner(deps, info, update),
            ExecuteMsg::SetRoute {
                denom_in,
                denom_out,
                route,
            } => self.set_route(deps, info.sender, denom_in, denom_out, route),
            ExecuteMsg::SwapExactIn {
                coin_in,
                denom_out,
                slippage,
                route,
            } => self.swap_exact_in(deps, env, info, coin_in, denom_out, slippage, route),
            ExecuteMsg::TransferResult {
                recipient,
                denom_in,
                denom_out,
            } => self.transfer_result(deps, env, info, recipient, denom_in, denom_out),
            ExecuteMsg::UpdateConfig {
                config,
            } => self.update_config(deps, info, config),
        }
    }

    pub fn query(&self, deps: Deps<Q>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
        let res = match msg {
            QueryMsg::Owner {} => to_json_binary(&self.owner.query(deps.storage)?),
            QueryMsg::EstimateExactInSwap {
                coin_in,
                denom_out,
                route,
            } => {
                to_json_binary(&self.estimate_exact_in_swap(deps, env, coin_in, denom_out, route)?)
            }
            QueryMsg::Route {
                denom_in,
                denom_out,
            } => to_json_binary(&self.query_route(deps, denom_in, denom_out)?),
            QueryMsg::Routes {
                start_after,
                limit,
            } => to_json_binary(&self.query_routes(deps, start_after, limit)?),
            QueryMsg::Config {} => to_json_binary(&self.query_config(deps)?),
        };
        res.map_err(Into::into)
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
            route: self.get_route(deps, &denom_in, &denom_out)?,
        })
    }

    fn query_routes(
        &self,
        deps: Deps<Q>,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> ContractResult<RoutesResponse<R>> {
        let start = start_after.map(Bound::exclusive);
        paginate_map(&self.routes, deps.storage, start, limit, |(denom_in, denom_out), route| {
            Ok(RouteResponse {
                denom_in,
                denom_out,
                route,
            })
        })
    }

    fn query_config(&self, deps: Deps<Q>) -> ContractResult<Option<C>> {
        let config = self.config.may_load(deps.storage)?;
        Ok(config)
    }

    fn estimate_exact_in_swap(
        &self,
        deps: Deps<Q>,
        env: Env,
        coin_in: Coin,
        denom_out: String,
        route: Option<SwapperRoute>,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let config = self.query_config(deps)?;

        // if route is not provided, use the default route from state
        let route = match route {
            Some(route) => R::from(route, config)?,
            None => self.get_route(deps, &coin_in.denom, &denom_out)?,
        };
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
        route: Option<SwapperRoute>,
    ) -> ContractResult<Response<M>> {
        let max_slippage = Decimal::percent(MAX_SLIPPAGE_PERCENTAGE);
        if slippage > max_slippage {
            return Err(ContractError::MaxSlippageExceeded {
                max_slippage,
                slippage,
            });
        }

        // if route is not provided, use the default route from state
        let route = match route {
            Some(route) => {
                let config = self.query_config(deps.as_ref())?;

                R::from(route, config)?
            }
            None => self
                .routes
                .load(deps.storage, (coin_in.denom.clone(), denom_out.clone()))
                .map_err(|_| ContractError::NoRoute {
                    from: coin_in.denom.clone(),
                    to: denom_out.clone(),
                })?,
        };

        let swap_msg = route.build_exact_in_swap_msg(&deps.querier, &env, &coin_in, slippage)?;

        // Check balance of result of swapper and send back result to sender
        let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_json_binary(&ExecuteMsg::<R, C>::TransferResult {
                recipient: info.sender,
                denom_in: coin_in.denom.clone(),
                denom_out: denom_out.clone(),
            })?,
        });

        Ok(Response::new()
            .add_message(swap_msg)
            .add_message(transfer_msg)
            .add_attribute("action", "swap_fn")
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
            return Err(ContractError::Unauthorized {
                user: info.sender.to_string(),
                action: "transfer result".to_string(),
            });
        };

        let denom_in_balance =
            deps.querier.query_balance(env.contract.address.clone(), denom_in)?;
        let denom_out_balance = deps.querier.query_balance(env.contract.address, denom_out)?;

        let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: [denom_in_balance, denom_out_balance]
                .iter()
                .filter(|c| !c.amount.is_zero())
                .cloned()
                .collect(),
        });

        Ok(Response::new().add_attribute("action", "transfer_result").add_message(transfer_msg))
    }

    fn set_route(
        &self,
        deps: DepsMut<Q>,
        sender: Addr,
        denom_in: String,
        denom_out: String,
        route: R,
    ) -> ContractResult<Response<M>> {
        self.owner.assert_owner(deps.storage, &sender)?;

        route.validate(&deps.querier, &denom_in, &denom_out)?;

        self.routes.save(deps.storage, (denom_in.clone(), denom_out.clone()), &route)?;

        Ok(Response::new()
            .add_attribute("action", "rover/base/set_route")
            .add_attribute("denom_in", denom_in)
            .add_attribute("denom_out", denom_out)
            .add_attribute("route", route.to_string()))
    }

    fn get_route(&self, deps: Deps<Q>, denom_in: &str, denom_out: &str) -> ContractResult<R> {
        self.routes.load(deps.storage, (denom_in.to_string(), denom_out.to_string())).map_err(
            |_| ContractError::NoRoute {
                from: denom_in.to_string(),
                to: denom_out.to_string(),
            },
        )
    }

    fn update_owner(
        &self,
        deps: DepsMut<Q>,
        info: MessageInfo,
        update: OwnerUpdate,
    ) -> ContractResult<Response<M>> {
        Ok(self.owner.update(deps, info, update)?)
    }

    fn update_config(
        &self,
        deps: DepsMut<Q>,
        info: MessageInfo,
        config: C,
    ) -> ContractResult<Response<M>> {
        self.owner.assert_owner(deps.storage, &info.sender)?;

        config.validate(deps.api)?;
        self.config.save(deps.storage, &config)?;

        Ok(Response::new().add_attribute("action", "rover/base/update_config"))
    }
}
