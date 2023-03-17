use cosmwasm_std::{Addr, StdError::NotFound};
use mars_owner::OwnerError::NotEmergencyOwner;
use mars_rover::{
    adapters::vault::VaultUnchecked,
    error::ContractError::{InvalidConfig, Owner, Std},
    msg::execute::EmergencyUpdate,
};

use crate::helpers::{
    assert_err, locked_vault_info, uatom_info, unlocked_vault_info, uosmo_info, MockEnv,
};

pub mod helpers;

#[test]
fn only_emergency_owner_can_invoke_emergency_powers() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(&emergency_owner).build().unwrap();
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.emergency_update(&bad_guy, EmergencyUpdate::DisallowCoin("uosmo".to_string()));
    assert_err(res, Owner(NotEmergencyOwner {}))
}

#[test]
fn not_callable_if_no_emergency_role_set() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.emergency_update(&bad_guy, EmergencyUpdate::DisallowCoin("uosmo".to_string()));
    assert_err(res, Owner(NotEmergencyOwner {}))
}

#[test]
fn emergency_owner_can_blacklist_coins() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let mut mock = MockEnv::new()
        .emergency_owner(&emergency_owner)
        .allowed_coins(&[osmo_info.clone(), atom_info.clone()])
        .build()
        .unwrap();

    let allowed_coins_before = mock.query_allowed_coins(None, None);
    assert_eq!(allowed_coins_before.len(), 2);

    mock.emergency_update(&emergency_owner, EmergencyUpdate::DisallowCoin(osmo_info.denom))
        .unwrap();

    let allowed_coins_after = mock.query_allowed_coins(None, None);
    assert_eq!(allowed_coins_after.len(), 1);
    assert_eq!(allowed_coins_after.first().unwrap(), &atom_info.denom)
}

#[test]
fn raises_if_coin_does_not_exist() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let osmo_info = uosmo_info();

    let mut mock = MockEnv::new()
        .emergency_owner(&emergency_owner)
        .allowed_coins(&[osmo_info])
        .build()
        .unwrap();

    let res =
        mock.emergency_update(&emergency_owner, EmergencyUpdate::DisallowCoin("xyz".to_string()));

    assert_err(
        res,
        InvalidConfig {
            reason: "xyz not in config".to_string(),
        },
    )
}

#[test]
fn emergency_owner_can_drop_vault_max_ltv() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let vault_a = locked_vault_info();
    let vault_b = unlocked_vault_info();

    let mut mock = MockEnv::new()
        .emergency_owner(&emergency_owner)
        .vault_configs(&[vault_a.clone(), vault_b.clone()])
        .build()
        .unwrap();

    let vault_a_addr = mock.get_vault(&vault_a);
    let vault_b_addr = mock.get_vault(&vault_b);

    let vault_a_config_before = mock.query_vault_config(&vault_a_addr).unwrap();
    assert!(!vault_a_config_before.config.max_ltv.is_zero());
    let vault_b_config_before = mock.query_vault_config(&vault_b_addr).unwrap();
    assert!(!vault_b_config_before.config.max_ltv.is_zero());

    mock.emergency_update(&emergency_owner, EmergencyUpdate::SetZeroMaxLtv(vault_a_addr.clone()))
        .unwrap();

    let vault_a_config_after = mock.query_vault_config(&vault_a_addr).unwrap();
    assert!(vault_a_config_after.config.max_ltv.is_zero()); // Dropped to zero ✅
    let vault_b_config_after = mock.query_vault_config(&vault_b_addr).unwrap();
    assert!(!vault_b_config_after.config.max_ltv.is_zero());
}

#[test]
fn raises_if_vault_does_not_exist_for_max_ltv_drop() {
    let emergency_owner = Addr::unchecked("miles_morales");

    let mut mock = MockEnv::new().emergency_owner(&emergency_owner).build().unwrap();

    let res = mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::SetZeroMaxLtv(VaultUnchecked::new("vault_addr_123".to_string())),
    );

    assert_err(
        res,
        Std(NotFound {
            kind: "mars_rover::adapters::vault::config::VaultConfig".to_string(),
        }),
    )
}

#[test]
fn emergency_owner_can_drop_deposit_cap() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let vault_a = locked_vault_info();
    let vault_b = unlocked_vault_info();

    let mut mock = MockEnv::new()
        .emergency_owner(&emergency_owner)
        .vault_configs(&[vault_a.clone(), vault_b.clone()])
        .build()
        .unwrap();

    let vault_a_addr = mock.get_vault(&vault_a);
    let vault_b_addr = mock.get_vault(&vault_b);

    let vault_a_config_before = mock.query_vault_config(&vault_a_addr).unwrap();
    assert!(!vault_a_config_before.config.deposit_cap.amount.is_zero());
    let vault_b_config_before = mock.query_vault_config(&vault_b_addr).unwrap();
    assert!(!vault_b_config_before.config.deposit_cap.amount.is_zero());

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::SetZeroDepositCap(vault_a_addr.clone()),
    )
    .unwrap();

    let vault_a_config_after = mock.query_vault_config(&vault_a_addr).unwrap();
    assert!(vault_a_config_after.config.deposit_cap.amount.is_zero()); // Dropped to zero ✅
    let vault_b_config_after = mock.query_vault_config(&vault_b_addr).unwrap();
    assert!(!vault_b_config_after.config.deposit_cap.amount.is_zero());
}

#[test]
fn raises_if_vault_does_not_exist_for_deposit_cap_drop() {
    let emergency_owner = Addr::unchecked("miles_morales");

    let mut mock = MockEnv::new().emergency_owner(&emergency_owner).build().unwrap();

    let res = mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::SetZeroDepositCap(VaultUnchecked::new("vault_addr_123".to_string())),
    );

    assert_err(
        res,
        Std(NotFound {
            kind: "mars_rover::adapters::vault::config::VaultConfig".to_string(),
        }),
    )
}
