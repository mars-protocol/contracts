use cosmwasm_std::{Coin, Decimal, StdError, Uint128};
use mars_mock_red_bank::msg::CoinMarketInfo;
use mars_rover::{
    adapters::vault::{
        LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultUnlockingPosition,
    },
    msg::query::Positions,
};

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn raises_when_credit_manager_not_set() {
    let mock = MockEnv::new().skip_cm_config().build().unwrap();
    let err: StdError = mock.query_health("xyz").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: The credit manager address has not been set in config"
                .to_string()
        )
    );
}

#[test]
fn raises_with_non_existent_account_id() {
    let mock = MockEnv::new().build().unwrap();
    let err: StdError = mock.query_health("xyz").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: Generic error: Querier contract error: mars_rover::msg::query::Positions not found"
                .to_string()
        )
    );
}

#[test]
fn computes_correct_position_with_zero_assets() {
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
    );

    let health = mock.query_health(account_id).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::zero());
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

// Testable via only unlocking positions
#[test]
fn adds_vault_base_denoms_to_oracle_and_red_bank() {
    let mut mock = MockEnv::new().build().unwrap();

    let vault_base_token = "base_token_abc";
    let account_id = "123";

    let unlocking_amount = Uint128::new(22);

    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: Vault::new(mock.vault_contract.clone()),
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::zero()),
                    unlocking: UnlockingPositions::new(vec![VaultUnlockingPosition {
                        id: 1443,
                        coin: Coin {
                            denom: vault_base_token.to_string(),
                            amount: unlocking_amount,
                        },
                    }]),
                }),
            }],
        },
    );

    mock.set_allowed_coins(&[vault_base_token.to_string()]);

    let max_ltv = Decimal::from_atomics(4523u128, 4).unwrap();
    let liquidation_threshold = Decimal::from_atomics(5u128, 1).unwrap();

    mock.set_price(vault_base_token, Decimal::one());
    mock.set_market(
        vault_base_token,
        &CoinMarketInfo {
            denom: vault_base_token.to_string(),
            max_ltv,
            liquidation_threshold,
            liquidation_bonus: Decimal::from_atomics(9u128, 2).unwrap(),
        },
    );

    let health = mock.query_health(account_id).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, unlocking_amount);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        unlocking_amount.checked_mul_floor(max_ltv).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        unlocking_amount.checked_mul_floor(liquidation_threshold).unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

#[test]
fn allowed_coins_work() {
    let mut mock = MockEnv::new().build().unwrap();

    mock.set_allowed_coins(&[]);

    let umars = "umars";
    mock.set_price(umars, Decimal::one());

    let max_ltv = Decimal::from_atomics(4523u128, 4).unwrap();
    let liquidation_threshold = Decimal::from_atomics(5u128, 1).unwrap();

    mock.set_market(
        umars,
        &CoinMarketInfo {
            denom: umars.to_string(),
            max_ltv,
            liquidation_threshold,
            liquidation_bonus: Decimal::from_atomics(9u128, 2).unwrap(),
        },
    );

    let deposit_amount = Uint128::new(30);

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![Coin {
                denom: umars.to_string(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
    );

    let health = mock.query_health(account_id).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, deposit_amount); // price of 1
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero()); // coin not in whitelist
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        deposit_amount.checked_mul_floor(liquidation_threshold).unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    // Add to whitelist
    mock.set_allowed_coins(&[umars.to_string()]);
    let health = mock.query_health(account_id).unwrap();
    // Now reflects deposit value
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        deposit_amount.checked_mul_floor(max_ltv).unwrap()
    );
}

#[test]
fn vault_whitelist_affects_max_ltv() {
    let mut mock = MockEnv::new().build().unwrap();

    let vault_base_token = "base_token_abc";
    let account_id = "123";

    let vault_token_amount = Uint128::new(1_000_000);
    let base_token_amount = Uint128::new(100);

    mock.deposit_into_vault(base_token_amount);

    let vault = Vault::new(mock.vault_contract.clone());

    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(vault_token_amount)),
            }],
        },
    );
    mock.set_allowed_coins(&[vault_base_token.to_string()]);

    let max_ltv = Decimal::from_atomics(4523u128, 4).unwrap();
    let liquidation_threshold = Decimal::from_atomics(5u128, 1).unwrap();

    mock.set_price(vault_base_token, Decimal::one());
    mock.set_market(
        vault_base_token,
        &CoinMarketInfo {
            denom: vault_base_token.to_string(),
            max_ltv,
            liquidation_threshold,
            liquidation_bonus: Decimal::from_atomics(9u128, 2).unwrap(),
        },
    );

    let vault_config = mock.query_vault_config(&vault.clone().into());
    let vault_max_ltv = vault_config.config.max_ltv;
    let vault_liq_threshold = vault_config.config.liquidation_threshold;

    let health = mock.query_health(account_id).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, base_token_amount);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        base_token_amount.checked_mul_floor(vault_max_ltv).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        base_token_amount.checked_mul_floor(vault_liq_threshold).unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    // After de-listing, maxLTV drops to zero
    mock.vault_allowed(&vault.into(), false);
    let health = mock.query_health(account_id).unwrap();
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
}
