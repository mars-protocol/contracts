use crate::multitest::contracts::{
    mock_address_provider_contract, mock_incentives_contract, mock_oracle_osmosis_contract,
    mock_red_bank_contract, mock_rewards_collector_osmosis_contract,
};
use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::{BasicApp, Executor};
use mars_outpost::red_bank::CreateOrUpdateConfig;
use mars_outpost::{address_provider, oracle, red_bank};
use mars_outpost::{incentives, rewards_collector};

pub fn deploy_address_provider(app: &mut BasicApp) -> Addr {
    let code_id = app.store_code(mock_address_provider_contract());

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

pub fn deploy_incentives(app: &mut BasicApp) -> Addr {
    let code_id = app.store_code(mock_incentives_contract());

    let address_provider = deploy_address_provider(app);

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

pub fn deploy_oracle_osmosis(app: &mut BasicApp) -> Addr {
    let code_id = app.store_code(mock_oracle_osmosis_contract());

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &oracle::InstantiateMsg {
            owner: owner.to_string(),
            base_denom: "uosmo".to_string(),
        },
        &[],
        "oracle",
        None,
    )
    .unwrap()
}

pub fn deploy_red_bank(app: &mut BasicApp) -> Addr {
    let code_id = app.store_code(mock_red_bank_contract());

    let address_provider = deploy_address_provider(app);

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &red_bank::InstantiateMsg {
            config: CreateOrUpdateConfig {
                owner: Some(owner.to_string()),
                address_provider: Some(address_provider.to_string()),
                close_factor: Some(Decimal::percent(90)),
            },
        },
        &[],
        "red-bank",
        None,
    )
    .unwrap()
}

pub fn deploy_rewards_collector_osmosis(app: &mut BasicApp) -> Addr {
    let code_id = app.store_code(mock_rewards_collector_osmosis_contract());

    let address_provider = deploy_address_provider(app);

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &rewards_collector::InstantiateMsg {
            owner: owner.to_string(),
            address_provider: address_provider.to_string(),
            safety_tax_rate: Decimal::percent(5),
            safety_fund_denom: "umars".to_string(),
            fee_collector_denom: "uosmo".to_string(),
            channel_id: "1".to_string(),
            timeout_revision: 0,
            timeout_blocks: 0,
            timeout_seconds: 0,
            slippage_tolerance: Decimal::percent(5),
        },
        &[],
        "rewards-collector",
        None,
    )
    .unwrap()
}
