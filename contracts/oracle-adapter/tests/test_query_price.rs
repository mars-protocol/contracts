use std::ops::{Div, Mul};

use cosmwasm_std::{Empty, Uint128};
use cosmwasm_vault_standard::VaultStandardQueryMsg::{PreviewRedeem, TotalVaultTokenSupply};
use cw_multi_test::App;

use mars_oracle_adapter::msg::{ConfigResponse, QueryMsg, VaultPricingInfo};
use mars_outpost::oracle::PriceResponse;
use mars_rover::traits::IntoDecimal;

use crate::helpers::{instantiate_oracle_adapter, mock_vault_info};

pub mod helpers;

#[test]
fn test_non_vault_coin_priced() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);

    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let uosmo_oracle_res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            config.oracle.address().to_string(),
            &QueryMsg::Price {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    let res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Price {
                denom: "uosmo".to_string(),
            },
        )
        .unwrap();

    assert_eq!(res.price, uosmo_oracle_res.price);
}

#[test]
fn test_vault_coin_preview_redeem() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let vault_info = mock_vault_info();

    let vault_info: VaultPricingInfo = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::PricingInfo {
                denom: vault_info.vault_coin_denom,
            },
        )
        .unwrap();

    let vault_token_supply: Uint128 = app
        .wrap()
        .query_wasm_smart(vault_info.addr.clone(), &TotalVaultTokenSupply::<Empty> {})
        .unwrap();

    let total_lp_tokens: Uint128 = app
        .wrap()
        .query_wasm_smart(
            vault_info.addr,
            &PreviewRedeem::<Empty> {
                amount: vault_token_supply,
            },
        )
        .unwrap();

    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let lp_token_oracle_res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            config.oracle.address().to_string(),
            &QueryMsg::Price {
                denom: vault_info.base_denom.clone(),
            },
        )
        .unwrap();

    let total_value_of_vault = total_lp_tokens
        .to_dec()
        .unwrap()
        .mul(lp_token_oracle_res.price);

    let price_per_vault_coin = total_value_of_vault.div(vault_token_supply);

    let oracle_adapter_res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::Price {
                denom: vault_info.vault_coin_denom,
            },
        )
        .unwrap();

    // vault token price = total lp tokens in vault * price of lp token / total vault tokens issued
    //    This formula can't be used in production because the first multiplication results in an
    //    integer that exceeds the memory allocated to u128's. But for this test it's a good check
    //    on our current formula where we use:
    // Decimal::from_ratio(total_underlying, vault_coin_supply) * price of lp token
    //    This method does not cause an overflow given Decimal::from_ratio casts to u256
    assert_eq!(oracle_adapter_res.price, price_per_vault_coin)
}
