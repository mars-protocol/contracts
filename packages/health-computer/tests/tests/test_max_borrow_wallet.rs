use std::collections::HashMap;

use cosmwasm_std::{coin, Uint128};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{AccountKind, BorrowTarget},
};

use super::helpers::{udai_info, umars_info};

#[test]
fn max_borrow_wallet_offset_good() {
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(udai.denom.clone(), udai.price)]),
        params: HashMap::from([(udai.denom.clone(), udai.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![coin(1200, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_borrow_amount =
        h.max_borrow_amount_estimate(&udai.denom, &BorrowTarget::Wallet).unwrap();
    assert_eq!(Uint128::new(1014), max_borrow_amount);
}

#[test]
fn max_borrow_wallet_offset_margin_of_error() {
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.denom.clone(), umars.price)]),
        params: HashMap::from([(umars.denom.clone(), umars.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![coin(1200, &umars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_borrow_amount =
        h.max_borrow_amount_estimate(&umars.denom, &BorrowTarget::Wallet).unwrap();

    // Normally could be 960, but conservative offset rounding has a margin of error
    assert_eq!(Uint128::new(959), max_borrow_amount);
}
