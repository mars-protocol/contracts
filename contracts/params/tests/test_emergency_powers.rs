use cosmwasm_std::Addr;
use mars_owner::OwnerError;
use mars_params::{
    error::ContractError::Owner,
    types::{
        AssetParamsUpdate, EmergencyUpdate, RedBankEmergencyUpdate, RoverEmergencyUpdate,
        VaultConfigUpdate,
    },
};

use crate::helpers::{assert_err, default_asset_params, default_vault_config, MockEnv};

pub mod helpers;

#[test]
fn only_owner_can_invoke_emergency_powers() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::RedBank(RedBankEmergencyUpdate::DisableBorrowing("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::DisallowCoin("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::SetZeroDepositCapOnVault("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::SetZeroMaxLtvOnVault("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));
}

#[test]
fn disabling_borrowing() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let mut params = default_asset_params();
    params.red_bank.borrow_enabled = true;

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            denom: denom.clone(),
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(params.red_bank.borrow_enabled);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::RedBank(RedBankEmergencyUpdate::DisableBorrowing(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.red_bank.borrow_enabled);
}

#[test]
fn disallow_coin() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let mut params = default_asset_params();
    params.rover.whitelisted = true;

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            denom: denom.clone(),
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(params.rover.whitelisted);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::DisallowCoin(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.rover.whitelisted);
}

#[test]
fn set_zero_max_ltv() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let vault = "vault_addr_123".to_string();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            addr: vault.clone(),
            config: default_vault_config(),
        },
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(!params.max_loan_to_value.is_zero());

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::SetZeroMaxLtvOnVault(vault.clone())),
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(params.max_loan_to_value.is_zero());
}

#[test]
fn set_zero_deposit_cap() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let vault = "vault_addr_123".to_string();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            addr: vault.clone(),
            config: default_vault_config(),
        },
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(!params.deposit_cap.amount.is_zero());

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Rover(RoverEmergencyUpdate::SetZeroDepositCapOnVault(vault.clone())),
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(params.deposit_cap.amount.is_zero());
}
