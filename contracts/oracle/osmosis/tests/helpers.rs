#![allow(dead_code)]

use std::marker::PhantomData;

use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{coin, from_binary, Deps, DepsMut, OwnedDeps};
use osmo_bindings::{OsmosisQuery, PoolStateResponse};

use mars_outpost::oracle::{InstantiateMsg, QueryMsg};
use mars_testing::{mock_info, MarsMockQuerier};

use mars_oracle_osmosis::contract::entry;
use mars_oracle_osmosis::msg::ExecuteMsg;
use mars_oracle_osmosis::OsmosisPriceSource;

pub fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier, OsmosisQuery> {
    let mut deps = OwnedDeps::<_, _, _, OsmosisQuery> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[])),
        custom_query_type: PhantomData,
    };

    // set a few osmosis pools
    deps.querier.set_pool_state(
        1,
        PoolStateResponse {
            assets: vec![coin(42069, "uatom"), coin(69420, "uosmo")],
            shares: coin(10000, "gamm/pool/1"),
        },
    );
    deps.querier.set_pool_state(
        64,
        PoolStateResponse {
            assets: vec![coin(12345, "uusdc"), coin(23456, "uatom")],
            shares: coin(10000, "gamm/pool/64"),
        },
    );
    deps.querier.set_pool_state(
        89,
        PoolStateResponse {
            assets: vec![coin(12345, "uosmo"), coin(88888, "umars")],
            shares: coin(10000, "gamm/pool/89"),
        },
    );
    deps.querier.set_pool_state(
        3333,
        PoolStateResponse {
            assets: vec![coin(100000, "uusdc"), coin(100000, "uusdt"), coin(100000, "udai")],
            shares: coin(10000, "gamm/pool/3333"),
        },
    );

    // instantiate the oracle contract
    entry::instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "uosmo".to_string(),
        },
    )
    .unwrap();

    deps
}

pub fn set_price_source(
    deps: DepsMut<OsmosisQuery>,
    denom: &str,
    price_source: OsmosisPriceSource,
) {
    entry::execute(
        deps,
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: denom.to_string(),
            price_source,
        },
    )
    .unwrap();
}

pub fn query<T: serde::de::DeserializeOwned>(deps: Deps<OsmosisQuery>, msg: QueryMsg) -> T {
    from_binary(&entry::query(deps, mock_env(), msg).unwrap()).unwrap()
}
