use std::ops::{Div, Mul};

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

    let lp_token_price = Decimal::from_atomics(8745u128, 2).unwrap();
    let lp_token_in_vault = Uint128::new(120_042);
    let vaults_value = lp_token_in_vault.to_dec().unwrap().mul(lp_token_price);

    let price_per_vault_coin = vaults_value.div(STARTING_VAULT_SHARES);

    assert_eq!(res.price, price_per_vault_coin)
}
