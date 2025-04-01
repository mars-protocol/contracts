use cosmwasm_std::{coin, Decimal, Empty, Uint128};
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_types::{
    address_provider::{
        ExecuteMsg as ExecuteMsgAddr, InstantiateMsg as InstantiateAddr, MarsAddressType,
    },
    oracle,
    rewards_collector::{
        ExecuteMsg, InstantiateMsg as InstantiateRewards, RewardConfig, TransferType, UpdateConfig,
    },
    swapper::{EstimateExactInSwapResponse, OsmoRoute, OsmoSwap, QueryMsg, SwapperRoute},
};
use osmosis_test_tube::{Account, Gamm, Module, OsmosisTestApp, Wasm};

use crate::{
    cosmos_bank::Bank,
    helpers::{
        osmosis::{assert_err, instantiate_contract},
        swap_to_create_twap_records,
    },
};

mod cosmos_bank;
mod helpers;

const OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME: &str = "mars-address-provider";
const OSMOSIS_REWARDS_CONTRACT_NAME: &str = "mars-rewards-collector-osmosis";
const OSMOSIS_SWAPPER_CONTRACT_NAME: &str = "mars-swapper-osmosis";
const OSMOSIS_ORACLE_CONTRACT_NAME: &str = "mars-oracle-osmosis";

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

    let usdc_denom = "uusdc";
    let mars_denom = "umars";
    let safety_fund_denom = usdc_denom;
    let revenue_share_denom = usdc_denom;
    let fee_collector_denom = "umars";
    let safety_tax_rate = Decimal::percent(25);
    let revenue_share_tax_rate = Decimal::percent(10);
    let rewards_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_REWARDS_CONTRACT_NAME,
        &InstantiateRewards {
            owner: signer.address(),
            address_provider: addr_provider_addr.clone(),
            safety_tax_rate,
            revenue_share_tax_rate,
            safety_fund_config: RewardConfig {
                target_denom: safety_fund_denom.to_string(),
                transfer_type: TransferType::Bank,
            },
            revenue_share_config: RewardConfig {
                target_denom: revenue_share_denom.to_string(),
                transfer_type: TransferType::Bank,
            },
            fee_collector_config: RewardConfig {
                target_denom: fee_collector_denom.to_string(),
                transfer_type: TransferType::Ibc,
            },
            channel_id: "channel-1".to_string(),
            timeout_seconds: 60,
            slippage_tolerance: Decimal::percent(5),
        },
    );

    // Instantiate swapper addr
    let swapper_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_SWAPPER_CONTRACT_NAME,
        &mars_types::swapper::InstantiateMsg {
            owner: signer.address(),
        },
    );

    // Instantiate oracle addr
    let oracle_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_ORACLE_CONTRACT_NAME,
        &mars_types::oracle::InstantiateMsg::<Empty> {
            owner: signer.address(),
            base_denom: usdc_denom.to_string(),
            custom_init: None,
        },
    );

    // Set swapper addr in address provider
    wasm.execute(
        &addr_provider_addr,
        &mars_types::address_provider::ExecuteMsg::SetAddress {
            address_type: MarsAddressType::Swapper,
            address: swapper_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    // Set oracle addr in address provider
    wasm.execute(
        &addr_provider_addr,
        &mars_types::address_provider::ExecuteMsg::SetAddress {
            address_type: MarsAddressType::Oracle,
            address: oracle_addr.clone(),
        },
        &[],
        signer,
    )
    .unwrap();

    // Set prices in the oracle
    wasm.execute(
        &oracle_addr,
        &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: usdc_denom.to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: mars_denom.to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(30u128, 20u128),
            },
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uatom".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(50u128, 20u128),
            },
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &oracle_addr,
        &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
            denom: "uosmo".to_string(),
            price_source: OsmosisPriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(10u128, 20u128),
            },
        },
        &[],
        signer,
    )
    .unwrap();

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

    let safety_fund_amt_swap =
        Uint128::new(osmo_balance) * (safety_tax_rate + revenue_share_tax_rate);
    let fee_collector_amt_swap = Uint128::new(osmo_balance) - safety_fund_amt_swap;

    let safety_fund_route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: pool_usdc_osmo,
            to: safety_fund_denom.to_string(),
        }],
    }));
    let safety_fund_estimate: EstimateExactInSwapResponse = wasm
        .query(
            &swapper_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(safety_fund_amt_swap.u128(), "uosmo"),
                denom_out: safety_fund_denom.to_string(),
                route: safety_fund_route.clone(),
            },
        )
        .unwrap();

    let safety_fund_min_receive = safety_fund_estimate.amount * Decimal::percent(99);

    let fee_collector_route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: pool_mars_osmo,
            to: fee_collector_denom.to_string(),
        }],
    }));
    let fee_collector_estimate: EstimateExactInSwapResponse = wasm
        .query(
            &swapper_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(fee_collector_amt_swap.u128(), "uosmo"),
                denom_out: fee_collector_denom.to_string(),
                route: fee_collector_route.clone(),
            },
        )
        .unwrap();
    let fee_collector_min_receive = fee_collector_estimate.amount * Decimal::percent(99);

    // swap osmo
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SwapAsset {
            denom: "uosmo".to_string(),
            amount: None,
            safety_fund_route,
            fee_collector_route,
            safety_fund_min_receive: Some(safety_fund_min_receive),
            fee_collector_min_receive: Some(fee_collector_min_receive),
        },
        &[],
        signer,
    )
    .unwrap();

    let safety_fund_amt_swap =
        Uint128::new(atom_balance) * (safety_tax_rate + revenue_share_tax_rate);
    let fee_collector_amt_swap = Uint128::new(atom_balance) - safety_fund_amt_swap;

    let safety_fund_route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![
            OsmoSwap {
                pool_id: pool_atom_osmo,
                to: "uosmo".to_string(),
            },
            OsmoSwap {
                pool_id: pool_usdc_osmo,
                to: safety_fund_denom.to_string(),
            },
        ],
    }));
    let safety_fund_estimate: EstimateExactInSwapResponse = wasm
        .query(
            &swapper_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(safety_fund_amt_swap.u128(), "uatom"),
                denom_out: safety_fund_denom.to_string(),
                route: safety_fund_route.clone(),
            },
        )
        .unwrap();
    let safety_fund_min_receive = safety_fund_estimate.amount * Decimal::percent(99);

    let fee_collector_route = Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![
            OsmoSwap {
                pool_id: pool_atom_osmo,
                to: "uosmo".to_string(),
            },
            OsmoSwap {
                pool_id: pool_mars_osmo,
                to: fee_collector_denom.to_string(),
            },
        ],
    }));
    let fee_collector_estimate: EstimateExactInSwapResponse = wasm
        .query(
            &swapper_addr,
            &QueryMsg::EstimateExactInSwap {
                coin_in: coin(fee_collector_amt_swap.u128(), "uatom"),
                denom_out: fee_collector_denom.to_string(),
                route: fee_collector_route.clone(),
            },
        )
        .unwrap();
    let fee_collector_min_receive = fee_collector_estimate.amount * Decimal::percent(99);

    // swap atom
    println!("second swap");
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::SwapAsset {
            denom: "uatom".to_string(),
            amount: None,
            safety_fund_route,
            fee_collector_route,
            safety_fund_min_receive: Some(safety_fund_min_receive),
            fee_collector_min_receive: Some(fee_collector_min_receive),
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

#[test]
fn distribute_rewards_if_ibc_channel_invalid() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(
            &[
                coin(1_000_000_000_000, "uusdc"),
                coin(1_000_000_000_000, "umars"),
                coin(1_000_000_000_000, "uosmo"), // for gas
            ],
            2,
        )
        .unwrap();
    let signer = &accs[0];
    let user = &accs[1];

    // setup address-provider contract
    let addr_provider_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_ADDR_PROVIDER_CONTRACT_NAME,
        &InstantiateAddr {
            owner: signer.address(),
            prefix: "osmo".to_string(),
        },
    );
    wasm.execute(
        &addr_provider_addr,
        &ExecuteMsgAddr::SetAddress {
            address_type: MarsAddressType::FeeCollector,
            address: "osmo17xfxz0axs6cr7jejqpphuhs7yldnp295acmu9a".to_string(),
        },
        &[],
        signer,
    )
    .unwrap();
    wasm.execute(
        &addr_provider_addr,
        &ExecuteMsgAddr::SetAddress {
            address_type: MarsAddressType::SafetyFund,
            address: "osmo1f2m24wktq0sw3c0lexlg7fv4kngwyttvzws3a3r3al9ld2s2pvds87jqvf".to_string(),
        },
        &[],
        signer,
    )
    .unwrap();

    wasm.execute(
        &addr_provider_addr,
        &ExecuteMsgAddr::SetAddress {
            address_type: MarsAddressType::RevenueShare,
            address: "osmo14qncu5xag9ec26cx09x6pwncn9w74pq3wyr8rj".to_string(),
        },
        &[],
        signer,
    )
    .unwrap();

    // setup rewards-collector contract
    let safety_fund_denom = "uusdc";
    let fee_collector_denom = "umars";
    let revenue_share_denom = "uusdc";
    let rewards_addr = instantiate_contract(
        &wasm,
        signer,
        OSMOSIS_REWARDS_CONTRACT_NAME,
        &InstantiateRewards {
            owner: signer.address(),
            address_provider: addr_provider_addr,
            safety_tax_rate: Decimal::percent(50),
            revenue_share_tax_rate: Decimal::percent(10),
            safety_fund_config: RewardConfig {
                target_denom: safety_fund_denom.to_string(),
                transfer_type: TransferType::Bank,
            },
            revenue_share_config: RewardConfig {
                target_denom: revenue_share_denom.to_string(),
                transfer_type: TransferType::Bank,
            },
            fee_collector_config: RewardConfig {
                target_denom: fee_collector_denom.to_string(),
                transfer_type: TransferType::Ibc,
            },
            channel_id: "".to_string(),
            timeout_seconds: 60,
            slippage_tolerance: Decimal::percent(1),
        },
    );

    // fund rewards-collector contract
    let bank = Bank::new(&app);
    let usdc_funded = 800_000_000u128;
    let mars_funded = 50_000_000u128;
    bank.send(user, &rewards_addr, &[coin(usdc_funded, "uusdc")]).unwrap();
    bank.send(user, &rewards_addr, &[coin(mars_funded, "umars")]).unwrap();
    let usdc_balance = bank.query_balance(&rewards_addr, "uusdc");
    assert_eq!(usdc_balance, usdc_funded);
    let mars_balance = bank.query_balance(&rewards_addr, "umars");
    assert_eq!(mars_balance, mars_balance);

    // distribute umars rewards
    let res = wasm
        .execute(
            &rewards_addr,
            &ExecuteMsg::DistributeRewards {
                denom: "umars".to_string(),
            },
            &[],
            signer,
        )
        .unwrap_err();
    assert_err(res, "invalid source channel ID: identifier cannot be blank: invalid identifier");

    // update ibc channel
    wasm.execute(
        &rewards_addr,
        &ExecuteMsg::UpdateConfig {
            new_cfg: UpdateConfig {
                address_provider: None,
                safety_tax_rate: None,
                revenue_share_tax_rate: None,
                safety_fund_config: None,
                revenue_share_config: None,
                fee_collector_config: None,
                channel_id: Some("channel-1".to_string()),
                timeout_seconds: None,
                slippage_tolerance: None,
            },
        },
        &[],
        signer,
    )
    .unwrap();

    // distribute rewards
    let res = wasm
        .execute(
            &rewards_addr,
            &ExecuteMsg::DistributeRewards {
                denom: "umars".to_string(),
            },
            &[],
            signer,
        )
        .unwrap_err();
    assert_err(res, "port ID (transfer) channel ID (channel-1): channel not found");
}
