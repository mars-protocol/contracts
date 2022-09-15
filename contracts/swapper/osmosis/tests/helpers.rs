use std::fmt::Debug;

use anyhow::Result as AnyResult;
use cosmwasm_std::Addr;
use cosmwasm_std::CustomQuery;
use cw_multi_test::{AppResponse, Executor};
use cw_multi_test::{Contract, ContractWrapper};
use osmo_bindings::{OsmosisMsg, OsmosisQuery};
use osmo_bindings_test::OsmosisApp;
use schemars::JsonSchema;

use rover::adapters::swap::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};
use swapper_base::ContractError;
use swapper_osmosis::contract::{execute, instantiate, query};
use swapper_osmosis::route::OsmosisRoute;

pub fn mock_osmosis_app() -> OsmosisApp {
    OsmosisApp::default()
}

pub fn mock_osmosis_contract<C, Q>() -> Box<dyn Contract<C, Q>>
where
    C: Clone + Debug + PartialEq + JsonSchema,
    Q: CustomQuery,
    ContractWrapper<
        ExecuteMsg<OsmosisRoute>,
        Config<String>,
        QueryMsg,
        ContractError,
        ContractError,
        ContractError,
        OsmosisMsg,
        OsmosisQuery,
    >: Contract<C, Q>,
{
    let contract = ContractWrapper::new(execute, instantiate, query); //.with_reply(reply);
    Box::new(contract)
}

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn instantiate_contract(app: &mut OsmosisApp) -> Addr {
    let owner = Addr::unchecked("owner");
    let contract = mock_osmosis_contract();
    let code_id = app.store_code(contract);
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &InstantiateMsg {
            owner: owner.to_string(),
        },
        &[],
        "mock-swapper-osmosis-contract",
        None,
    )
    .unwrap()
}
