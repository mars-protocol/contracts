use cosmwasm_std::{Decimal, Uint128};
use mars_incentives::state::INCENTIVE_STATES;
use mars_red_bank_types::incentives::{IncentiveState, IncentiveStateResponse, QueryMsg};

use crate::helpers::th_setup;

mod helpers;

#[test]
fn query_incentive_state() {
    let mut deps = th_setup();

    // incentives
    let uosmo_incentive = IncentiveState {
        // emission_per_second: Uint128::new(100),
        // start_time: 120,
        // duration: 8640000,
        index: Decimal::one(),
        last_updated: 150,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uosmo", "umars"), &uosmo_incentive).unwrap();
    let uatom_incentive = IncentiveState {
        // emission_per_second: Uint128::zero(),
        // start_time: 0,
        // duration: 1200,
        index: Decimal::one(),
        last_updated: 1000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uatom", "umars"), &uatom_incentive).unwrap();
    let uusdc_incentive = IncentiveState {
        // emission_per_second: Uint128::new(200),
        // start_time: 12000,
        // duration: 86400,
        index: Decimal::from_ratio(120u128, 50u128),
        last_updated: 120000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uusdc", "umars"), &uusdc_incentive).unwrap();

    let res: IncentiveStateResponse = helpers::th_query(
        deps.as_ref(),
        QueryMsg::IncentiveState {
            collateral_denom: "uatom".to_string(),
            incentive_denom: "umars".to_string(),
        },
    );
    assert_eq!(
        res,
        IncentiveStateResponse::from("uatom".to_string(), "umars".to_string(), uatom_incentive)
    );
}

#[test]
fn query_incentive_states() {
    let mut deps = th_setup();

    // incentives
    let uosmo_incentive = IncentiveState {
        // emission_per_second: Uint128::new(100),
        // start_time: 120,
        // duration: 8640000,
        index: Decimal::one(),
        last_updated: 150,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uosmo", "umars"), &uosmo_incentive).unwrap();
    let uatom_incentive = IncentiveState {
        // emission_per_second: Uint128::zero(),
        // start_time: 0,
        // duration: 1200,
        index: Decimal::one(),
        last_updated: 1000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uatom", "umars"), &uatom_incentive).unwrap();
    let uusdc_incentive = IncentiveState {
        // emission_per_second: Uint128::new(200),
        // start_time: 12000,
        // duration: 86400,
        index: Decimal::from_ratio(120u128, 50u128),
        last_updated: 120000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uusdc", "umars"), &uusdc_incentive).unwrap();

    // NOTE: responses are ordered alphabetically by denom
    let res: Vec<IncentiveStateResponse> = helpers::th_query(
        deps.as_ref(),
        QueryMsg::IncentiveStates {
            start_after_collateral_denom: None,
            start_after_incentive_denom: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            IncentiveStateResponse::from("uatom".to_string(), "umars".to_string(), uatom_incentive),
            IncentiveStateResponse::from(
                "uosmo".to_string(),
                "umars".to_string(),
                uosmo_incentive.clone()
            ),
            IncentiveStateResponse::from("uusdc".to_string(), "umars".to_string(), uusdc_incentive),
        ]
    );

    // NOTE: responses are ordered alphabetically by denom
    let res: Vec<IncentiveStateResponse> = helpers::th_query(
        deps.as_ref(),
        QueryMsg::IncentiveStates {
            start_after_collateral_denom: Some("uatom".to_string()),
            start_after_incentive_denom: None,
            limit: Some(1),
        },
    );
    assert_eq!(
        res,
        vec![IncentiveStateResponse::from(
            "uosmo".to_string(),
            "umars".to_string(),
            uosmo_incentive
        )]
    );
}
