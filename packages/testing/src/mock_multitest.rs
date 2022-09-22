use cosmwasm_std::Addr;
use cw_multi_test::{BasicApp, ContractWrapper, Executor};
use mars_outpost::address_provider;
use mars_outpost::incentives;

pub fn instantiate_address_provider(app: &mut BasicApp) -> Addr {
    let contract = Box::new(ContractWrapper::new(
        mars_address_provider::contract::execute,
        mars_address_provider::contract::instantiate,
        mars_address_provider::contract::query,
    ));
    let code_id = app.store_code(contract);

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &address_provider::InstantiateMsg {
            owner: owner.to_string(),
            prefix: "chain".to_string(),
        },
        &[],
        "address-provider",
        None,
    )
    .unwrap()
}

pub fn instantiate_incentives(app: &mut BasicApp) -> Addr {
    let contract = Box::new(ContractWrapper::new(
        mars_incentives::contract::execute,
        mars_incentives::contract::instantiate,
        mars_incentives::contract::query,
    ));
    let code_id = app.store_code(contract);

    let address_provider = instantiate_address_provider(app);

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &incentives::InstantiateMsg {
            owner: owner.to_string(),
            address_provider: address_provider.to_string(),
            mars_denom: "umars".to_string(),
        },
        &[],
        "incentives",
        None,
    )
    .unwrap()
}
