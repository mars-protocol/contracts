use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, Binary, CustomQuery, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw_storage_plus::{Bound, Item, Map};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_owner::{Owner, OwnerUpdate};

use mars_outpost::oracle::{
    Config, ConfigResponse, ExecuteMsg, InstantiateMsg, PriceResponse, PriceSourceResponse,
    QueryMsg,
};

use crate::error::ContractResult;
use crate::PriceSource;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub struct OracleBase<'a, P, C>
where
    P: PriceSource<C>,
    C: CustomQuery,
{
    /// Contract's owner
    pub owner: Owner<'a>,
    /// The contract's config
    pub config: Item<'a, Config>,
    /// The price source of each coin denom
    pub price_sources: Map<'a, String, P>,
    /// Phantom data holds the custom query type
    pub custom_query: PhantomData<C>,
}

impl<'a, P, C> Default for OracleBase<'a, P, C>
where
    P: PriceSource<C>,
    C: CustomQuery,
{
    fn default() -> Self {
        Self {
            owner: Owner::new("owner"),
            config: Item::new("config"),
            price_sources: Map::new("price_sources"),
            custom_query: PhantomData,
        }
    }
}

impl<'a, P, C> OracleBase<'a, P, C>
where
    P: PriceSource<C>,
    C: CustomQuery,
{
    pub fn instantiate(&self, deps: DepsMut<C>, msg: InstantiateMsg) -> ContractResult<Response> {
        self.owner.initialize(
            deps.storage,
            deps.api,
            SetInitialOwner {
                owner: msg.owner,
            },
        )?;

        self.config.save(
            deps.storage,
            &Config {
                base_denom: msg.base_denom,
            },
        )?;

        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut<C>,
        info: MessageInfo,
        msg: ExecuteMsg<P>,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::UpdateOwner(update) => self.update_owner(deps, info, update),
            ExecuteMsg::SetPriceSource {
                denom,
                price_source,
            } => self.set_price_source(deps, info.sender, denom, price_source),
            ExecuteMsg::RemovePriceSource {
                denom,
            } => self.remove_price_source(deps, info.sender, denom),
        }
    }

    pub fn query(&self, deps: Deps<C>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Config {} => to_binary(&self.query_config(deps)?),
            QueryMsg::PriceSource {
                denom,
            } => to_binary(&self.query_price_source(deps, denom)?),
            QueryMsg::PriceSources {
                start_after,
                limit,
            } => to_binary(&self.query_price_sources(deps, start_after, limit)?),
            QueryMsg::Price {
                denom,
            } => to_binary(&self.query_price(deps, env, denom)?),
            QueryMsg::Prices {
                start_after,
                limit,
            } => to_binary(&self.query_prices(deps, env, start_after, limit)?),
        }
    }

    fn update_owner(
        &self,
        deps: DepsMut<C>,
        info: MessageInfo,
        update: OwnerUpdate,
    ) -> ContractResult<Response> {
        Ok(self.owner.update(deps, info, update)?)
    }

    fn set_price_source(
        &self,
        deps: DepsMut<C>,
        sender_addr: Addr,
        denom: String,
        price_source: P,
    ) -> ContractResult<Response> {
        self.owner.assert_owner(deps.storage, &sender_addr)?;

        let cfg = self.config.load(deps.storage)?;
        price_source.validate(&deps.querier, &denom, &cfg.base_denom)?;
        self.price_sources.save(deps.storage, denom.clone(), &price_source)?;

        Ok(Response::new()
            .add_attribute("action", "outposts/oracle/set_price_source")
            .add_attribute("denom", denom)
            .add_attribute("price_source", price_source.to_string()))
    }

    fn remove_price_source(
        &self,
        deps: DepsMut<C>,
        sender_addr: Addr,
        denom: String,
    ) -> ContractResult<Response> {
        self.owner.assert_owner(deps.storage, &sender_addr)?;

        self.price_sources.remove(deps.storage, denom.clone());

        Ok(Response::new()
            .add_attribute("action", "outposts/oracle/remove_price_source")
            .add_attribute("denom", denom))
    }

    fn query_config(&self, deps: Deps<C>) -> StdResult<ConfigResponse> {
        let owner_state = self.owner.query(deps.storage)?;
        let cfg = self.config.load(deps.storage)?;
        Ok(ConfigResponse {
            owner: owner_state.owner,
            proposed_new_owner: owner_state.proposed,
            base_denom: cfg.base_denom,
        })
    }

    fn query_price_source(
        &self,
        deps: Deps<C>,
        denom: String,
    ) -> StdResult<PriceSourceResponse<P>> {
        Ok(PriceSourceResponse {
            denom: denom.clone(),
            price_source: self.price_sources.load(deps.storage, denom)?,
        })
    }

    fn query_price_sources(
        &self,
        deps: Deps<C>,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<PriceSourceResponse<P>>> {
        let start = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

        self.price_sources
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (k, v) = item?;
                Ok(PriceSourceResponse {
                    denom: k,
                    price_source: v,
                })
            })
            .collect()
    }

    fn query_price(&self, deps: Deps<C>, env: Env, denom: String) -> StdResult<PriceResponse> {
        let cfg = self.config.load(deps.storage)?;
        let price_source = self.price_sources.load(deps.storage, denom.clone())?;
        Ok(PriceResponse {
            denom: denom.clone(),
            price: price_source.query_price(&deps, &env, &denom, &cfg.base_denom)?,
        })
    }

    fn query_prices(
        &self,
        deps: Deps<C>,
        env: Env,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<PriceResponse>> {
        let cfg = self.config.load(deps.storage)?;

        let start = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

        self.price_sources
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (k, v) = item?;
                Ok(PriceResponse {
                    denom: k.clone(),
                    price: v.query_price(&deps, &env, &k, &cfg.base_denom)?,
                })
            })
            .collect()
    }
}
