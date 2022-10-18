use cosmwasm_std::{coin, Coin, Uint128};
use cw_multi_test::App;

use mars_oracle_adapter::msg::QueryMsg;

use crate::helpers::{instantiate_oracle_adapter, mock_vault_info};

pub mod helpers;

#[test]
fn test_non_vault_coin_underlying() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);

    let coins: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::PriceableUnderlying {
                coin: coin(100, "uosmo"),
            },
        )
        .unwrap();

    assert_eq!(coins.len(), 1);
    assert_eq!(coins[0].denom, "uosmo".to_string());
    assert_eq!(coins[0].amount, Uint128::new(100));
}

#[test]
fn test_vault_coin_preview_redeem() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let vault_info = mock_vault_info();

    let coins: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::PriceableUnderlying {
                coin: coin(1000, vault_info.vault_coin_denom),
            },
        )
        .unwrap();

    assert_eq!(coins.len(), 2);
    assert_eq!(coins[0].denom, "uatom".to_string());
    assert_eq!(coins[0].amount, Uint128::new(32));
    assert_eq!(coins[1].denom, "uosmo".to_string());
    assert_eq!(coins[1].amount, Uint128::new(120));
}
