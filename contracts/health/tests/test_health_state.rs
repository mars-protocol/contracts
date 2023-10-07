use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_params::msg::AssetParamsUpdate::AddOrUpdate;
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::msg::query::{DebtAmount, Positions};
use mars_rover_health_types::{AccountKind, HealthState};

use crate::helpers::{default_asset_params, MockEnv};

pub mod helpers;

#[test]
fn zero_debts_results_in_healthy_state() {
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = "1352524";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![Coin {
                denom: "xyz".to_string(),
                amount: Uint128::one(),
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
    );

    let state =
        mock.query_health_state(account_id, AccountKind::Default, ActionKind::Default).unwrap();

    assert_eq!(state, HealthState::Healthy);
}

#[test]
fn computing_health_when_healthy() {
    let mut mock = MockEnv::new().build().unwrap();

    let umars = "umars";
    mock.set_price(umars, Decimal::one(), ActionKind::Default);
    mock.update_asset_params(AddOrUpdate {
        params: default_asset_params(umars),
    });

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![Coin {
                denom: umars.to_string(),
                amount: Uint128::new(100),
            }],
            debts: vec![DebtAmount {
                denom: umars.to_string(),
                shares: Default::default(),
                amount: Uint128::new(30),
            }],
            lends: vec![],
            vaults: vec![],
        },
    );

    let state =
        mock.query_health_state(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert_eq!(state, HealthState::Healthy);
}

#[test]
fn computing_health_when_unhealthy() {
    let mut mock = MockEnv::new().build().unwrap();

    let umars = "umars";
    mock.set_price(umars, Decimal::one(), ActionKind::Default);
    mock.update_asset_params(AddOrUpdate {
        params: default_asset_params(umars),
    });

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            deposits: vec![Coin {
                denom: umars.to_string(),
                amount: Uint128::new(100),
            }],
            debts: vec![DebtAmount {
                denom: umars.to_string(),
                shares: Default::default(),
                amount: Uint128::new(250),
            }],
            lends: vec![],
            vaults: vec![],
        },
    );

    let state =
        mock.query_health_state(account_id, AccountKind::Default, ActionKind::Default).unwrap();
    assert!(matches!(state, HealthState::Unhealthy { .. }));
}
