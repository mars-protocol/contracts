use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env},
    Uint128,
};
use mars_burn_contract::{contract::query, state::BURNT_AMOUNTS};
use mars_types::burn::{BurntAmountsResponse, QueryMsg};

#[test]
fn query_all_burnt_amounts() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Record some burnt amounts
    BURNT_AMOUNTS.save(deps.as_mut().storage, "token1", &Uint128::new(100)).unwrap();
    BURNT_AMOUNTS.save(deps.as_mut().storage, "token2", &Uint128::new(200)).unwrap();
    BURNT_AMOUNTS.save(deps.as_mut().storage, "token3", &Uint128::new(300)).unwrap();

    // Query all burnt amounts
    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::GetAllBurntAmounts {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let burnt_amounts: BurntAmountsResponse = from_json(res).unwrap();

    assert_eq!(burnt_amounts.burnt_amounts.len(), 3);
    assert_eq!(burnt_amounts.burnt_amounts[0].denom, "token1");
    assert_eq!(burnt_amounts.burnt_amounts[0].amount, Uint128::new(100));
    assert_eq!(burnt_amounts.burnt_amounts[1].denom, "token2");
    assert_eq!(burnt_amounts.burnt_amounts[1].amount, Uint128::new(200));
    assert_eq!(burnt_amounts.burnt_amounts[2].denom, "token3");
    assert_eq!(burnt_amounts.burnt_amounts[2].amount, Uint128::new(300));
}
