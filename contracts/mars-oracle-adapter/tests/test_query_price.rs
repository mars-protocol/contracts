use std::ops::{Add, Div, Mul};

use cosmwasm_std::{Decimal, Uint128};
use cw_multi_test::App;
use mars_outpost::oracle::PriceResponse;

use mars_oracle_adapter::msg::QueryMsg;
use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::traits::IntoDecimal;

use crate::helpers::{instantiate_oracle_adapter, mock_vault_info};

pub mod helpers;

#[test]
fn test_non_vault_coin_priced() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);

    let res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Price {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(res.price, Decimal::from_atomics(25u128, 2).unwrap())
}

#[test]
fn test_vault_coin_preview_redeem() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let vault_info = mock_vault_info();

    let res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Price {
                denom: vault_info.vault_coin_denom,
            },
        )
        .unwrap();

    let uatom_price = Decimal::from_atomics(10u128, 1).unwrap();
    let atom_in_vault = Uint128::new(32_343);
    let vaults_atom_value = atom_in_vault.to_dec().unwrap().mul(uatom_price);

    let uosmo_price = Decimal::from_atomics(25u128, 2).unwrap();
    let osmo_in_vault = Uint128::new(120_042);
    let vaults_osmo_value = osmo_in_vault.to_dec().unwrap().mul(uosmo_price);

    let price_per_vault_coin = vaults_atom_value
        .add(vaults_osmo_value)
        .div(STARTING_VAULT_SHARES);

    assert_eq!(res.price, price_per_vault_coin)
}
