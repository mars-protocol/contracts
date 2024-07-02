use astroport_v5::asset::Asset;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use mars_incentives::state::{
    ASTRO_INCENTIVE_STATES, ASTRO_TOTAL_LP_DEPOSITS, ASTRO_USER_LP_DEPOSITS, EMISSIONS,
    INCENTIVE_STATES,
};
use mars_testing::{mock_env, MockEnvParams};
use mars_types::incentives::{
    ActiveEmission, EmissionResponse, IncentiveState, IncentiveStateResponse,
    PaginatedStakedLpResponse, QueryMsg, StakedLpPositionResponse,
};
use test_case::test_case;

use super::helpers::{th_query, th_query_with_env, th_setup};

#[test]
fn query_incentive_state() {
    let mut deps = th_setup();

    // incentives
    let uosmo_incentive = IncentiveState {
        index: Decimal::one(),
        last_updated: 150,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uosmo", "umars"), &uosmo_incentive).unwrap();
    let uatom_incentive = IncentiveState {
        index: Decimal::one(),
        last_updated: 1000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uatom", "umars"), &uatom_incentive).unwrap();
    let uusdc_incentive = IncentiveState {
        index: Decimal::from_ratio(120u128, 50u128),
        last_updated: 120000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uusdc", "umars"), &uusdc_incentive).unwrap();

    let res: IncentiveStateResponse = th_query(
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
        index: Decimal::one(),
        last_updated: 150,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uosmo", "umars"), &uosmo_incentive).unwrap();
    let uatom_incentive = IncentiveState {
        index: Decimal::one(),
        last_updated: 1000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uatom", "umars"), &uatom_incentive).unwrap();
    let uusdc_incentive = IncentiveState {
        index: Decimal::from_ratio(120u128, 50u128),
        last_updated: 120000,
    };
    INCENTIVE_STATES.save(deps.as_mut().storage, ("uusdc", "umars"), &uusdc_incentive).unwrap();

    // NOTE: responses are ordered alphabetically by denom
    let res: Vec<IncentiveStateResponse> = th_query(
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
    let res: Vec<IncentiveStateResponse> = th_query(
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

#[test]
fn query_emission() {
    let mut deps = th_setup();

    EMISSIONS.save(deps.as_mut().storage, ("uosmo", "umars", 604800), &Uint128::new(100)).unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, ("uosmo", "umars", 604800 * 2), &Uint128::new(50))
        .unwrap();

    // Query before emission start
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 0,
        },
    );
    assert_eq!(res, Uint128::zero());

    // Query at timestamp of first emission start
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800,
        },
    );
    assert_eq!(res, Uint128::new(100));

    // Query at timestamp of second emission start
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800 * 2,
        },
    );
    assert_eq!(res, Uint128::new(50));

    // Query one second before second emission start
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800 * 2 - 1,
        },
    );
    assert_eq!(res, Uint128::new(100));

    // Query at timestamp some time into second emission start
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800 * 2 + 100,
        },
    );
    assert_eq!(res, Uint128::new(50));

    // Query the second before emission end
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800 * 3 - 1,
        },
    );
    assert_eq!(res, Uint128::new(50));

    // Query the second after emission end
    let res: Uint128 = th_query(
        deps.as_ref(),
        QueryMsg::Emission {
            collateral_denom: "uosmo".to_string(),
            incentive_denom: "umars".to_string(),
            timestamp: 604800 * 3,
        },
    );
    assert_eq!(res, Uint128::zero());
}

#[test]
fn query_emissions() {
    let mut deps = th_setup();

    EMISSIONS.save(deps.as_mut().storage, ("uusdc", "umars", 0), &Uint128::new(200)).unwrap();
    EMISSIONS.save(deps.as_mut().storage, ("uusdc", "umars", 604800), &Uint128::new(100)).unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, ("uusdc", "umars", 604800 * 2), &Uint128::new(50))
        .unwrap();

    let res: Vec<EmissionResponse> = th_query(
        deps.as_ref(),
        QueryMsg::Emissions {
            collateral_denom: "uusdc".to_string(),
            incentive_denom: "umars".to_string(),
            start_after_timestamp: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            EmissionResponse::from((0, Uint128::new(200))),
            EmissionResponse::from((604800, Uint128::new(100))),
            EmissionResponse::from((604800 * 2, Uint128::new(50))),
        ]
    );

    let res: Vec<EmissionResponse> = th_query(
        deps.as_ref(),
        QueryMsg::Emissions {
            collateral_denom: "uusdc".to_string(),
            incentive_denom: "umars".to_string(),
            start_after_timestamp: Some(100),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            EmissionResponse::from((604800, Uint128::new(100))),
            EmissionResponse::from((604800 * 2, Uint128::new(50))),
        ]
    );

    let res: Vec<EmissionResponse> = th_query(
        deps.as_ref(),
        QueryMsg::Emissions {
            collateral_denom: "uusdc".to_string(),
            incentive_denom: "umars".to_string(),
            start_after_timestamp: Some(604800),
            limit: Some(1),
        },
    );
    assert_eq!(res, vec![EmissionResponse::from((604800 * 2, Uint128::new(50)))]);
}

#[test_case(0 => Vec::<(String, Uint128)>::new() ; "query before emission start")]
#[test_case(604800 => vec![("uosmo".to_string(), 100u128.into())] ; "query at emission start time")]
#[test_case(604800 + 100 => vec![("uosmo".to_string(), 100u128.into())] ; "query during first emission")]
#[test_case(604800 * 2 => vec![
        ("umars".to_string(), 50u128.into()),
        ("uosmo".to_string(), 100u128.into())
    ]; "query at second emission start time")]
#[test_case(604800 * 2 + 100 => vec![
        ("umars".to_string(), 50u128.into()),
        ("uosmo".to_string(), 100u128.into())
    ]; "query during second emission")]
#[test_case(604800 * 3 => Vec::<(String, Uint128)>::new() ; "query at emission end time")]
#[test_case(604800 * 3 + 100 => Vec::<(String, Uint128)>::new() ; "query after emission end time")]
fn query_active_emissions(query_at_time: u64) -> Vec<(String, Uint128)> {
    let mut deps = th_setup();

    // Setup incentive states
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            ("uusdc", "uosmo"),
            &IncentiveState {
                index: Decimal::zero(),
                last_updated: 0,
            },
        )
        .unwrap();
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            ("uusdc", "umars"),
            &IncentiveState {
                index: Decimal::zero(),
                last_updated: 0,
            },
        )
        .unwrap();

    // Setup emissions
    EMISSIONS.save(deps.as_mut().storage, ("uusdc", "uosmo", 604800), &Uint128::new(100)).unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, ("uusdc", "umars", 604800 * 2), &Uint128::new(50))
        .unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, ("uusdc", "uosmo", 604800 * 2), &Uint128::new(100))
        .unwrap();

    th_query_with_env::<Vec<ActiveEmission>>(
        deps.as_ref(),
        mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(query_at_time),
            ..Default::default()
        }),
        QueryMsg::ActiveEmissions {
            collateral_denom: "uusdc".to_string(),
        },
    )
    .into_iter()
    .map(|x| (x.denom, x.emission_rate))
    .collect()
}

#[test]
fn query_staked_lp_positions_with_pagination() {
    let mut deps = th_setup();

    let astroport_incentives_addr = Addr::unchecked("astroport_incentives");
    deps.querier.set_astroport_incentives_address(astroport_incentives_addr.clone());

    let mut count = 0;
    let account_id = "1".to_string();

    // save 50 different deposits
    while count < 50 {
        count += 1;
        ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, (&account_id, &count.to_string()), &Uint128::new(count))
            .unwrap();
    }

    let page_1: PaginatedStakedLpResponse = th_query(
        deps.as_ref(),
        QueryMsg::StakedAstroLpPositions {
            account_id: account_id.clone(),
            start_after: None,
            limit: Some(51),
        },
    );

    // ensure we cap to 10
    assert_eq!(page_1.data.len(), 10);

    let page_2: PaginatedStakedLpResponse = th_query(
        deps.as_ref(),
        QueryMsg::StakedAstroLpPositions {
            account_id: account_id.clone(),
            start_after: page_1.data.last().map(|x| x.lp_coin.denom.clone()),
            limit: None,
        },
    );

    // Default length should be 5.
    assert_eq!(page_2.data.len(), 5);

    // Pages are sorted alphabetically (1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 2, 20)
    assert_eq!(page_2.data.first().map(|x| x.lp_coin.denom.clone()), Some("19".to_string()));
}

#[test]
fn query_staked_astro_lp_position() {
    let env = mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(1),
        ..Default::default()
    });
    let mut deps = th_setup();
    let account_id = "1".to_string();
    let lp_denom = "uastro".to_string();
    let reward_denom = "uusdc".to_string();
    let reward_amount = Uint128::new(100u128);
    let reward_coin = Coin {
        denom: reward_denom.clone(),
        amount: reward_amount,
    };

    let astroport_incentives_addr = Addr::unchecked("astroport_incentives");
    deps.querier.set_astroport_incentives_address(astroport_incentives_addr.clone());
    deps.querier.set_unclaimed_astroport_lp_rewards(
        &lp_denom,
        env.contract.address.as_ref(),
        vec![Asset::from(reward_coin.clone())],
    );
    ASTRO_TOTAL_LP_DEPOSITS.save(deps.as_mut().storage, &lp_denom, &Uint128::new(100u128)).unwrap();
    ASTRO_USER_LP_DEPOSITS
        .save(deps.as_mut().storage, (&account_id, &lp_denom), &Uint128::new(100u128))
        .unwrap();
    ASTRO_INCENTIVE_STATES
        .save(deps.as_mut().storage, (&lp_denom, &reward_denom), &Decimal::zero())
        .unwrap();

    let res: StakedLpPositionResponse = th_query(
        deps.as_ref(),
        QueryMsg::StakedAstroLpPosition {
            account_id: account_id.clone(),
            lp_denom: lp_denom.clone(),
        },
    );

    assert_eq!(res.lp_coin.denom, "uastro".to_string());
    assert_eq!(res.lp_coin.amount, Uint128::new(100u128));
    assert_eq!(res.rewards[0].amount, reward_coin.amount);
    assert_eq!(res.rewards[0].denom, reward_coin.denom);
    assert_eq!(res.rewards.len(), 1);
}
