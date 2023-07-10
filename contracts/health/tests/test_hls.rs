use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};
use mars_params::msg::AssetParamsUpdate::AddOrUpdate;
use mars_rover::{
    adapters::vault::{Vault, VaultAmount, VaultPosition, VaultPositionAmount},
    msg::query::{DebtAmount, Positions},
};
use mars_rover_health_types::AccountKind;

use crate::helpers::{default_asset_params, MockEnv};

pub mod helpers;

#[test]
fn hls_account_kind_passed_along() {
    let mut mock = MockEnv::new().build().unwrap();

    let vault_base_token = "base_token_abc";
    let debt_token = "umars";
    let account_id = "123";

    let vault_token_amount = Uint128::new(1_000_000);
    let base_token_amount = Uint128::new(100);

    mock.deposit_into_vault(base_token_amount);

    let vault = Vault::new(mock.vault_contract.clone());

    let positions = Positions {
        account_id: account_id.to_string(),
        deposits: vec![],
        debts: vec![DebtAmount {
            denom: debt_token.to_string(),
            shares: Uint128::new(10_000_000),
            amount: Uint128::new(50),
        }],
        lends: vec![],
        vaults: vec![VaultPosition {
            vault: vault.clone(),
            amount: VaultPositionAmount::Unlocked(VaultAmount::new(vault_token_amount)),
        }],
    };
    mock.set_positions_response(account_id, &positions);
    mock.set_price(debt_token, Decimal::one());
    mock.update_asset_params(AddOrUpdate {
        params: default_asset_params(debt_token),
    });

    mock.update_asset_params(AddOrUpdate {
        params: default_asset_params(vault_base_token),
    });

    mock.set_price(vault_base_token, Decimal::one());

    let vault_config = mock.query_vault_config(&vault.into());

    let health = mock.query_health_values(account_id, AccountKind::HighLeveredStrategy).unwrap();
    assert_eq!(health.total_debt_value, positions.debts.first().unwrap().amount);
    assert_eq!(health.total_collateral_value, base_token_amount);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        base_token_amount
            .checked_mul_floor(vault_config.hls.as_ref().unwrap().max_loan_to_value)
            .unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        base_token_amount
            .checked_mul_floor(vault_config.hls.unwrap().liquidation_threshold)
            .unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_str("1.2").unwrap())); // Default would have been 0.8
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_str("1.4").unwrap())); // Default would have been 1.2
    assert!(!health.above_max_ltv); // Default would have been above max_ltv
    assert!(!health.liquidatable);
}
