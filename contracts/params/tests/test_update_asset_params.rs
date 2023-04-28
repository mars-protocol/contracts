use cosmwasm_std::Addr;
use mars_owner::OwnerError;
use mars_params::{error::ContractError::Owner, types::AssetParamsUpdate};

use crate::helpers::{assert_contents_equal, assert_err, default_asset_params, MockEnv};

pub mod helpers;

#[test]
fn initial_state_of_params() {
    let mock = MockEnv::new().build().unwrap();
    let params = mock.query_all_asset_params(None, None);
    assert!(params.is_empty());
}

#[test]
fn only_owner_can_update_asset_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_asset_params(
        &bad_guy,
        AssetParamsUpdate::AddOrUpdate {
            denom: "xyz".to_string(),
            params: default_asset_params(),
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn initializing_asset_param() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();

    let params = default_asset_params();

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: params.clone(),
        },
    )
    .unwrap();

    let all_asset_params = mock.query_all_asset_params(None, None);
    assert_eq!(1, all_asset_params.len());
    let res = all_asset_params.first().unwrap();
    assert_eq!(&denom0, &res.denom);

    // Validate config set correctly
    assert_eq!(params.permissions.rover.whitelisted, res.params.permissions.rover.whitelisted);
    assert_eq!(
        params.permissions.red_bank.deposit_enabled,
        res.params.permissions.red_bank.deposit_enabled
    );
    assert_eq!(
        params.permissions.red_bank.borrow_enabled,
        res.params.permissions.red_bank.borrow_enabled
    );
    assert_eq!(params.max_loan_to_value, res.params.max_loan_to_value);
    assert_eq!(params.liquidation_threshold, res.params.liquidation_threshold);
    assert_eq!(params.liquidation_bonus, res.params.liquidation_bonus);
    assert_eq!(
        params.permissions.red_bank.deposit_cap,
        res.params.permissions.red_bank.deposit_cap
    );

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom1.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();

    let asset_params = mock.query_all_asset_params(None, None);
    assert_eq!(2, asset_params.len());
    assert_eq!(&denom1, &asset_params.get(1).unwrap().denom);
}

#[test]
fn add_same_denom_multiple_times() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();

    let asset_params = mock.query_all_asset_params(None, None);
    assert_eq!(1, asset_params.len());
    assert_eq!(denom0, asset_params.first().unwrap().denom);
}

#[test]
fn update_existing_asset_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();

    let mut params = default_asset_params();

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: params.clone(),
        },
    )
    .unwrap();

    let asset_params = mock.query_asset_params(&denom0);
    assert!(!asset_params.permissions.rover.whitelisted);
    assert!(asset_params.permissions.red_bank.deposit_enabled);

    params.permissions.rover.whitelisted = true;
    params.permissions.red_bank.deposit_enabled = false;

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params,
        },
    )
    .unwrap();

    let all_asset_params = mock.query_all_asset_params(None, None);
    assert_eq!(1, all_asset_params.len());

    let asset_params = mock.query_asset_params(&denom0);
    assert!(asset_params.permissions.rover.whitelisted);
    assert!(!asset_params.permissions.red_bank.deposit_enabled);
}

#[test]
fn removing_from_asset_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();
    let denom2 = "juno".to_string();

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0,
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom1,
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom2,
            params: default_asset_params(),
        },
    )
    .unwrap();

    let asset_params = mock.query_all_asset_params(None, None);
    assert_eq!(3, asset_params.len());
}

#[test]
fn pagination_query() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();
    let denom2 = "juno".to_string();
    let denom3 = "mars".to_string();
    let denom4 = "ion".to_string();
    let denom5 = "usdc".to_string();

    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom0.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom1.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom2.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom3.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom4.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();
    mock.update_asset_params(
        &owner,
        AssetParamsUpdate::AddOrUpdate {
            denom: denom5.to_string(),
            params: default_asset_params(),
        },
    )
    .unwrap();

    let asset_params_a = mock.query_all_asset_params(None, Some(2));
    let asset_params_b =
        mock.query_all_asset_params(asset_params_a.last().map(|r| r.denom.clone()), Some(2));
    let asset_params_c =
        mock.query_all_asset_params(asset_params_b.last().map(|r| r.denom.clone()), None);

    let combined = asset_params_a
        .iter()
        .cloned()
        .chain(asset_params_b.iter().cloned())
        .chain(asset_params_c.iter().cloned())
        .map(|r| r.denom)
        .collect::<Vec<_>>();

    assert_eq!(6, combined.len());

    assert_contents_equal(&[denom0, denom1, denom2, denom3, denom4, denom5], &combined)
}
