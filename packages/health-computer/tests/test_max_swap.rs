use std::collections::HashMap;

use cosmwasm_std::{coin, Uint128};
use mars_rover::msg::query::Positions;
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::{AccountKind, SwapKind};

use crate::helpers::{udai_info, umars_info};

pub mod helpers;

#[test]
fn max_swap_default() {
    let udai = udai_info();
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (udai.denom.clone(), udai.price),
            (umars.denom.clone(), umars.price),
        ]),
        params: HashMap::from([
            (udai.denom.clone(), udai.params.clone()),
            (umars.denom.clone(), umars.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_borrow_amount =
        h.max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Default).unwrap();
    assert_eq!(Uint128::new(1200), max_borrow_amount);
}

#[test]
fn max_swap_margin() {
    let udai = udai_info();
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (udai.denom.clone(), udai.price),
            (umars.denom.clone(), umars.price),
        ]),
        params: HashMap::from([
            (udai.denom.clone(), udai.params.clone()),
            (umars.denom.clone(), umars.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(5000, &udai.denom), coin(500, &umars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_borrow_amount =
        h.max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Margin).unwrap();
    assert_eq!(Uint128::new(31351), max_borrow_amount);
}
