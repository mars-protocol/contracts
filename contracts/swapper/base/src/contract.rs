use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, CustomMsg, CustomQuery, Decimal, Deps,
    DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw_paginate::paginate_map;
use cw_storage_plus::{Bound, Map};
use mars_owner::{Owner, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, RouteResponse,
    RoutesResponse,
};

use crate::{ContractError, ContractResult, Route};

pub struct SwapBase<'a, Q, M, R>
where
    Q: CustomQuery,
    M: CustomMsg,
    R: Route<M, Q>,
{
    /// The contract's owner who has special rights to update contract
    pub owner: Owner<'a>,
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
            owner: Owner::new("owner"),
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
        msg: ExecuteMsg<R>,
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
            QueryMsg::Owner {} => to_binary(&self.owner.query(deps.storage)?),
            QueryMsg::EstimateExactInSwap {
                coin_in,
                denom_out,
            } => to_binary(&self.estimate_exact_in_swap(deps, env, coin_in, denom_out)?),
            QueryMsg::Route {
                denom_in,
                denom_out,
            } => to_binary(&self.query_route(deps, denom_in, denom_out)?),
            QueryMsg::Routes {
                start_after,
                limit,
            } => to_binary(&self.query_routes(deps, start_after, limit)?),
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

    fn estimate_exact_in_swap(
        &self,
        deps: Deps<Q>,
        env: Env,
        coin_in: Coin,
        denom_out: String,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let route = self.get_route(deps, &coin_in.denom, &denom_out)?;
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
            .get_route(deps.as_ref(), &coin_in.denom, &denom_out)?
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
}
