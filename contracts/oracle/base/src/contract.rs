use std::marker::PhantomData;

use cosmwasm_std::{
    to_json_binary, Addr, Binary, CustomQuery, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult,
};
use cw_storage_plus::{Bound, Item, Map};
use mars_owner::{Owner, OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_types::oracle::{
    ActionKind, Config, ConfigResponse, ExecuteMsg, InstantiateMsg, PriceResponse,
    PriceSourceResponse, QueryMsg,
};
use mars_utils::helpers::validate_native_denom;

use crate::{error::ContractResult, ContractError, PriceSourceChecked, PriceSourceUnchecked};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub struct OracleBase<'a, P, PU, C, I, E>
where
    P: PriceSourceChecked<C>,
    PU: PriceSourceUnchecked<P, C>,
    C: CustomQuery,
{
    /// Contract's owner
    pub owner: Owner<'a>,
    /// The contract's config
    pub config: Item<'a, Config>,
    /// The price source of each coin denom
    pub price_sources: Map<'a, &'a str, P>,
    /// Phantom data holds the unchecked price source type
    pub unchecked_price_source: PhantomData<PU>,
    /// Phantom data holds the custom query type
    pub custom_query: PhantomData<C>,
    /// Phantom data holds the instantiate msg custom type
    pub instantiate_msg: PhantomData<I>,
    /// Phantom data holds the execute msg custom type
    pub execute_msg: PhantomData<E>,
}

impl<'a, P, PU, C, I, E> Default for OracleBase<'a, P, PU, C, I, E>
where
    P: PriceSourceChecked<C>,
    PU: PriceSourceUnchecked<P, C>,
    C: CustomQuery,
{
    fn default() -> Self {
        Self {
            owner: Owner::new("owner"),
            config: Item::new("config"),
            price_sources: Map::new("price_sources"),
            unchecked_price_source: PhantomData,
            custom_query: PhantomData,
            instantiate_msg: PhantomData,
            execute_msg: PhantomData,
        }
    }
}

impl<'a, P, PU, C, I, E> OracleBase<'a, P, PU, C, I, E>
where
    P: PriceSourceChecked<C>,
    PU: PriceSourceUnchecked<P, C>,
    C: CustomQuery,
{
    pub fn instantiate(
        &self,
        deps: DepsMut<C>,
        msg: InstantiateMsg<I>,
    ) -> ContractResult<Response> {
        validate_native_denom(&msg.base_denom)?;

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
        msg: ExecuteMsg<PU, E>,
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
            ExecuteMsg::UpdateConfig {
                base_denom,
            } => self.update_config(deps, info.sender, base_denom),
            // Custom messages should be handled by the implementing contract
            ExecuteMsg::Custom(_) => Err(ContractError::MissingCustomExecuteParams {}),
        }
    }

    pub fn query(&self, deps: Deps<C>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
        let res = match msg {
            QueryMsg::Config {} => to_json_binary(&self.query_config(deps)?),
            QueryMsg::PriceSource {
                denom,
            } => to_json_binary(&self.query_price_source(deps, denom)?),
            QueryMsg::PriceSources {
                start_after,
                limit,
            } => to_json_binary(&self.query_price_sources(deps, start_after, limit)?),
            QueryMsg::Price {
                denom,
                kind,
            } => to_json_binary(&self.query_price(
                deps,
                env,
                denom,
                kind.unwrap_or(ActionKind::Default),
            )?),
            QueryMsg::Prices {
                start_after,
                limit,
                kind,
            } => to_json_binary(&self.query_prices(
                deps,
                env,
                start_after,
                limit,
                kind.unwrap_or(ActionKind::Default),
            )?),
        };
        res.map_err(Into::into)
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
        price_source: PU,
    ) -> ContractResult<Response> {
        self.owner.assert_owner(deps.storage, &sender_addr)?;

        validate_native_denom(&denom)?;

        let cfg = self.config.load(deps.storage)?;
        let price_source =
            price_source.validate(&deps.as_ref(), &denom, &cfg.base_denom, &self.price_sources)?;
        self.price_sources.save(deps.storage, &denom, &price_source)?;

        Ok(Response::new()
            .add_attribute("action", "set_price_source")
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

        self.price_sources.remove(deps.storage, &denom);

        Ok(Response::new()
            .add_attribute("action", "remove_price_source")
            .add_attribute("denom", denom))
    }

    fn update_config(
        &self,
        deps: DepsMut<C>,
        sender_addr: Addr,
        base_denom: Option<String>,
    ) -> ContractResult<Response> {
        self.owner.assert_owner(deps.storage, &sender_addr)?;

        if let Some(bd) = &base_denom {
            validate_native_denom(bd)?;
        };

        let mut config = self.config.load(deps.storage)?;
        let prev_base_denom = config.base_denom.clone();
        config.base_denom = base_denom.unwrap_or(config.base_denom);
        self.config.save(deps.storage, &config)?;

        let response = Response::new()
            .add_attribute("action", "update_config")
            .add_attribute("prev_base_denom", prev_base_denom)
            .add_attribute("base_denom", config.base_denom);

        Ok(response)
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
            price_source: self.price_sources.load(deps.storage, &denom).map_err(|_| {
                StdError::generic_err(format!("No price source found for denom: {}", denom))
            })?,
            denom,
        })
    }

    fn query_price_sources(
        &self,
        deps: Deps<C>,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<PriceSourceResponse<P>>> {
        let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
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

    fn query_price(
        &self,
        deps: Deps<C>,
        env: Env,
        denom: String,
        kind: ActionKind,
    ) -> ContractResult<PriceResponse> {
        let cfg = self.config.load(deps.storage)?;

        let price_source = self.query_price_source(deps, denom.clone())?.price_source;

        Ok(PriceResponse {
            price: price_source.query_price(
                &deps,
                &env,
                &denom,
                &cfg,
                &self.price_sources,
                kind,
            )?,
            denom,
        })
    }

    fn query_prices(
        &self,
        deps: Deps<C>,
        env: Env,
        start_after: Option<String>,
        limit: Option<u32>,
        kind: ActionKind,
    ) -> ContractResult<Vec<PriceResponse>> {
        let cfg = self.config.load(deps.storage)?;

        let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

        self.price_sources
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (k, v) = item?;
                Ok(PriceResponse {
                    price: v.query_price(
                        &deps,
                        &env,
                        &k,
                        &cfg,
                        &self.price_sources,
                        kind.clone(),
                    )?,
                    denom: k,
                })
            })
            .collect()
    }
}
