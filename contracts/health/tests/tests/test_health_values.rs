use std::str::FromStr;

use cosmwasm_std::{Coin, Decimal, StdError, Uint128};
use mars_types::{
    adapters::vault::{
        LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultUnlockingPosition,
    },
    credit_manager::Positions,
    health::AccountKind,
    oracle::ActionKind,
    params::{
        AssetParamsUnchecked, AssetParamsUpdate::AddOrUpdate, CmSettings, HlsParamsUnchecked,
        LiquidationBonus, RedBankSettings, VaultConfigUpdate,
    },
};

use super::helpers::MockEnv;
use crate::tests::helpers::default_asset_params;

#[test]
fn raises_when_credit_manager_not_set() {
    let mock = MockEnv::new().skip_cm_config().build().unwrap();
    let err: StdError =
        mock.query_health_values("xyz", AccountKind::Default, ActionKind::Default).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: Generic error: Credit Manager contract is currently not set up in the health contract".to_string()
        )
    );
}

#[test]
fn raises_with_non_existent_account_id() {
    let mock = MockEnv::new().build().unwrap();
    let err: StdError =
        mock.query_health_values("xyz", AccountKind::Default, ActionKind::Default).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: Generic error: Querier contract error: type: mars_types::credit_manager::query::Positions; key: [00, 12, 70, 6F, 73, 69, 74, 69, 6F, 6E, 5F, 72, 65, 73, 70, 6F, 6E, 73, 65, 73, 78, 79, 7A] not found".to_string()
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
            account_kind: AccountKind::Default,
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
    );

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
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
            account_kind: AccountKind::Default,
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
            staked_astro_lps: vec![],
        },
    );

    let max_loan_to_value = Decimal::from_atomics(4523u128, 4).unwrap();
    let liquidation_threshold = Decimal::from_atomics(5u128, 1).unwrap();

    let update = AddOrUpdate {
        params: AssetParamsUnchecked {
            denom: vault_base_token.to_string(),
            credit_manager: CmSettings {
                whitelisted: true,
                hls: Some(HlsParamsUnchecked {
                    max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                    correlations: vec![],
                }),
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                borrow_enabled: false,
            },
            max_loan_to_value,
            liquidation_threshold,
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
            deposit_cap: Default::default(),
        },
    };

    mock.update_asset_params(update);

    mock.set_price(vault_base_token, Decimal::one(), ActionKind::Default);

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, unlocking_amount);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        unlocking_amount.checked_mul_floor(max_loan_to_value).unwrap()
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
fn whitelisted_coins_work() {
    let mut mock = MockEnv::new().build().unwrap();

    let umars = "umars";

    mock.set_price(umars, Decimal::one(), ActionKind::Default);

    let max_loan_to_value = Decimal::from_atomics(4523u128, 4).unwrap();
    let liquidation_threshold = Decimal::from_atomics(5u128, 1).unwrap();
    let liquidation_bonus = LiquidationBonus {
        starting_lb: Decimal::percent(1u64),
        slope: Decimal::from_atomics(2u128, 0).unwrap(),
        min_lb: Decimal::percent(2u64),
        max_lb: Decimal::percent(10u64),
    };

    let mut asset_params = AssetParamsUnchecked {
        denom: umars.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: Some(HlsParamsUnchecked {
                max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                correlations: vec![],
            }),
        },
        red_bank: RedBankSettings {
            deposit_enabled: false,
            borrow_enabled: false,
        },
        max_loan_to_value,
        liquidation_threshold,
        liquidation_bonus,
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Default::default(),
    };

    let update = AddOrUpdate {
        params: asset_params.clone(),
    };

    mock.update_asset_params(update);

    let deposit_amount = Uint128::new(30);

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![Coin {
                denom: umars.to_string(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
    );

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
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
    asset_params.credit_manager.whitelisted = true;
    mock.update_asset_params(AddOrUpdate {
        params: asset_params,
    });
    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    // Now reflects deposit value
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        deposit_amount.checked_mul_floor(max_loan_to_value).unwrap()
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
            account_kind: AccountKind::Default,
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(vault_token_amount)),
            }],
            staked_astro_lps: vec![],
        },
    );

    let update = AddOrUpdate {
        params: AssetParamsUnchecked {
            denom: vault_base_token.to_string(),
            credit_manager: CmSettings {
                whitelisted: true,
                hls: Some(HlsParamsUnchecked {
                    max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                    correlations: vec![],
                }),
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                borrow_enabled: false,
            },
            max_loan_to_value: Decimal::from_str("0.4523").unwrap(),
            liquidation_threshold: Decimal::from_str("0.5").unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
            deposit_cap: Default::default(),
        },
    };

    mock.update_asset_params(update);

    mock.set_price(vault_base_token, Decimal::one(), ActionKind::Default);

    let mut vault_config = mock.query_vault_config(&vault.into());

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, base_token_amount);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        base_token_amount.checked_mul_floor(vault_config.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        base_token_amount.checked_mul_floor(vault_config.liquidation_threshold).unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    // After de-listing, maxLTV drops to zero
    vault_config.whitelisted = false;

    mock.update_vault_params(VaultConfigUpdate::AddOrUpdate {
        config: vault_config.into(),
    });

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
}

#[test]
fn not_whitelisted_coins_work() {
    let mut mock = MockEnv::new().build().unwrap();

    let umars = "umars";
    let urandom = "urandom";

    mock.set_price(umars, Decimal::one(), ActionKind::Default);
    let umars_ap = default_asset_params(umars);
    mock.update_asset_params(AddOrUpdate {
        params: umars_ap.clone(),
    });

    let umars_deposit_amount = Uint128::new(30);
    let urandom_deposit_amount = Uint128::new(1200);

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![
                Coin {
                    denom: umars.to_string(),
                    amount: umars_deposit_amount,
                },
                Coin {
                    denom: urandom.to_string(),
                    amount: urandom_deposit_amount,
                },
            ],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
    );

    let health =
        mock.query_health_values(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.total_collateral_value, umars_deposit_amount); // price of 1
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        umars_deposit_amount.checked_mul_floor(umars_ap.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        umars_deposit_amount.checked_mul_floor(umars_ap.liquidation_threshold).unwrap()
    );
    assert_eq!(health.max_ltv_health_factor, None);
    assert_eq!(health.liquidation_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}
