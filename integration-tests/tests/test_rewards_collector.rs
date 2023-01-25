use cosmwasm_std::{coin, Decimal};
use mars_red_bank_types::{
    address_provider::InstantiateMsg as InstantiateAddr,
    rewards_collector::{ExecuteMsg, InstantiateMsg as InstantiateRewards},
};
use mars_rewards_collector_osmosis::OsmosisRoute;
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;
use osmosis_testing::{Account, Gamm, Module, OsmosisTestApp, Wasm};

use crate::{
    cosmos_bank::Bank,
    helpers::{osmosis::instantiate_contract, swap_to_create_twap_records},
};

mod cosmos_bank;
mod helpers;

const OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME: &str = "mars-address-provider";
const OSMOSIS_REWARDS_CONTRACT_NAME: &str = "mars-rewards-collector-osmosis";

#[test]
fn swapping_rewards() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(
            &[
                coin(1_000_000_000_000, "uatom"),
                coin(1_000_000_000_000, "umars"),
                coin(1_000_000_000_000, "uusdc"),
                coin(1_000_000_000_000, "uosmo"),
            ],
            2,
        )
        .unwrap();
    let signer = &accs[0];
    let user = &accs[1];

    let addr_provider_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME,
        &InstantiateAddr {
            owner: signer.address(),
            prefix: "osmo".to_string(),
        },
    );

    let safety_fund_denom = "uusdc";
    let fee_collector_denom = "umars";
    let rewards_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_REWARDS_CONTRACT_NAME,
        &InstantiateRewards {
            owner: signer.address(),
            address_provider: addr_provider_addr,
            safety_tax_rate: Decimal::percent(25),
            safety_fund_denom: safety_fund_denom.to_string(),
            fee_collector_denom: fee_collector_denom.to_string(),
            channel_id: "channel-1".to_string(),
            timeout_revision: 2,
            timeout_blocks: 10,
            timeout_seconds: 60,
            slippage_tolerance: Decimal::percent(1),
        },
    );

    let gamm = Gamm::new(&app);
    let pool_mars_osmo = gamm
        .create_basic_pool(&[coin(2_000_000, "umars"), coin(6_000_000, "uosmo")], signer)
        .unwrap()
        .data
        .pool_id;
    let pool_usdc_osmo = gamm
        .create_basic_pool(&[coin(500_000, "uusdc"), coin(1_000_000, "uosmo")], signer)
        .unwrap()
        .data
        .pool_id;
    let pool_atom_osmo = gamm
        .create_basic_pool(&[coin(200_000, "uatom"), coin(1_000_000, "uosmo")], signer)
        .unwrap()
        .data
        .pool_id;

    // swap to create historic index for TWAP
    swap_to_create_twap_records(
        &app,
        signer,
        pool_mars_osmo,
        coin(5u128, "umars"),
        "uosmo",
        600u64,
    );
    swap_to_create_twap_records(
        &app,
        signer,
        pool_usdc_osmo,
        coin(5u128, "uusdc"),
        "uosmo",
        600u64,
    );
    swap_to_create_twap_records(
        &app,
        signer,
        pool_atom_osmo,
        coin(5u128, "uatom"),
        "uosmo",
        600u64,
    );

    // set routes
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: safety_fund_denom.to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_usdc_osmo,
                token_out_denom: safety_fund_denom.to_string(),
            }]),
        },
        &[],
        signer,
    )
    .unwrap();
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uosmo".to_string(),
            denom_out: fee_collector_denom.to_string(),
            route: OsmosisRoute(vec![SwapAmountInRoute {
                pool_id: pool_mars_osmo,
                token_out_denom: fee_collector_denom.to_string(),
            }]),
        },
        &[],
        signer,
    )
    .unwrap();
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: safety_fund_denom.to_string(),
            route: OsmosisRoute(vec![
                SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
                SwapAmountInRoute {
                    pool_id: pool_usdc_osmo,
                    token_out_denom: safety_fund_denom.to_string(),
                },
            ]),
        },
        &[],
        signer,
    )
    .unwrap();
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SetRoute {
            denom_in: "uatom".to_string(),
            denom_out: fee_collector_denom.to_string(),
            route: OsmosisRoute(vec![
                SwapAmountInRoute {
                    pool_id: pool_atom_osmo,
                    token_out_denom: "uosmo".to_string(),
                },
                SwapAmountInRoute {
                    pool_id: pool_mars_osmo,
                    token_out_denom: fee_collector_denom.to_string(),
                },
            ]),
        },
        &[],
        signer,
    )
    .unwrap();

    // fund contract
    let bank = Bank::new(&app);
    bank.send(user, &rewards_addr, &[coin(125u128, "uosmo")]).unwrap();
    bank.send(user, &rewards_addr, &[coin(55u128, "uatom")]).unwrap();
    let osmo_balance = bank.query_balance(&rewards_addr, "uosmo");
    assert_eq!(osmo_balance, 125u128);
    let atom_balance = bank.query_balance(&rewards_addr, "uatom");
    assert_eq!(atom_balance, 55u128);
    let safety_fund_denom_balance = bank.query_balance(&rewards_addr, safety_fund_denom);
    assert_eq!(safety_fund_denom_balance, 0u128);
    let fee_collector_denom_balance = bank.query_balance(&rewards_addr, fee_collector_denom);
    assert_eq!(fee_collector_denom_balance, 0u128);

    // swap osmo
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::<OsmosisRoute>::SwapAsset {
            denom: "uosmo".to_string(),
            amount: None,
        },
        &[],
        signer,
    )
    .unwrap();

    // swap atom
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::<OsmosisRoute>::SwapAsset {
            denom: "uatom".to_string(),
            amount: None,
        },
        &[],
        signer,
    )
    .unwrap();

    // osmo and atom should be swapped to safety_fund_denom and fee_collector_denom
    let osmo_balance = bank.query_balance(&rewards_addr, "uosmo");
    assert_eq!(osmo_balance, 0u128);
    let atom_balance = bank.query_balance(&rewards_addr, "uatom");
    assert_eq!(atom_balance, 0u128);
    let safety_fund_denom_balance = bank.query_balance(&rewards_addr, safety_fund_denom);
    assert!(safety_fund_denom_balance > 0u128);
    let fee_collector_denom_balance = bank.query_balance(&rewards_addr, fee_collector_denom);
    assert!(fee_collector_denom_balance > 0u128);
}
