use std::collections::HashMap;

use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coin, from_binary, Addr, Decimal, Deps, OwnedDeps};

use osmo_bindings::{OsmosisQuery, PoolStateResponse, Step};

use mars_outpost::protocol_rewards_collector::{Config, QueryMsg};
use mars_outpost::testing::{mock_info, MarsMockQuerier};

use crate::{contract, msg::ExecuteMsg, SwapInstruction};

pub(super) fn mock_config() -> Config<Addr> {
    Config {
        owner: Addr::unchecked("owner"),
        address_provider: Addr::unchecked("address_provider"),
        safety_tax_rate: Decimal::percent(25),
        safety_fund_denom: "uusdc".to_string(),
        fee_collector_denom: "umars".to_string(),
        channel_id: "channel-69".to_string(),
        timeout_revision: 1,
        timeout_blocks: 50,
        timeout_seconds: 300,
    }
}

pub(super) fn mock_instructions() -> HashMap<(&'static str, &'static str), SwapInstruction> {
    let mut map = HashMap::new();

    // uosmo -> umars
    map.insert(
        ("uosmo", "umars"),
        SwapInstruction(vec![Step {
            pool_id: 420,
            denom_out: "umars".to_string(),
        }]),
    );

    // uatom -> uosmo -> umars
    map.insert(
        ("uatom", "umars"),
        SwapInstruction(vec![
            Step {
                pool_id: 1,
                denom_out: "uosmo".to_string(),
            },
            Step {
                pool_id: 420,
                denom_out: "umars".to_string(),
            },
        ]),
    );

    // uatom -> uosmo -> uusdc
    map.insert(
        ("uatom", "uusdc"),
        SwapInstruction(vec![
            Step {
                pool_id: 1,
                denom_out: "uosmo".to_string(),
            },
            Step {
                pool_id: 69,
                denom_out: "uusdc".to_string(),
            },
        ]),
    );

    map
}

pub(super) fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier, OsmosisQuery> {
    let mut deps = OwnedDeps::<_, _, _, OsmosisQuery> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[(
            MOCK_CONTRACT_ADDR,
            &[coin(88888, "uatom"), coin(1234, "uusdc"), coin(8964, "umars")],
        )])),
        custom_query_type: Default::default(),
    };

    // set up pools for the mock osmosis querier
    deps.querier.set_pool_state(
        1,
        PoolStateResponse {
            assets: vec![coin(1, "uatom"), coin(1, "uosmo")],
            shares: coin(1, "uLP"),
        },
    );
    deps.querier.set_pool_state(
        68,
        PoolStateResponse {
            assets: vec![coin(1, "uatom"), coin(1, "uusdc")],
            shares: coin(1, "uLP"),
        },
    );
    deps.querier.set_pool_state(
        69,
        PoolStateResponse {
            assets: vec![coin(1, "uosmo"), coin(1, "uusdc")],
            shares: coin(1, "uLP"),
        },
    );
    deps.querier.set_pool_state(
        420,
        PoolStateResponse {
            assets: vec![coin(1, "uosmo"), coin(1, "umars")],
            shares: coin(1, "uLP"),
        },
    );

    // instantiate the contract
    let info = mock_info("deployer");
    let msg = mock_config().into();
    contract::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // set a few swap instructions
    mock_instructions().into_iter().for_each(|((denom_in, denom_out), instruction)| {
        contract::execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetInstruction {
                denom_in: denom_in.to_string(),
                denom_out: denom_out.to_string(),
                instruction,
            },
        )
        .unwrap();
    });

    deps
}

pub(super) fn query<T: serde::de::DeserializeOwned>(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    msg: QueryMsg,
) -> T {
    from_binary(&contract::query(deps, mock_env(), msg).unwrap()).unwrap()
}
