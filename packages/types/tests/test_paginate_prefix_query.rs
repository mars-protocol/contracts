use cosmwasm_std::{testing::mock_dependencies, StdError, Uint128};
use cw_storage_plus::{Bound, Map};
use mars_types::paginate_prefix_query;

#[test]
pub fn empty_when_prefix_not_found() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<(&str, &str), Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, ("account_1", "denom_1"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_1", "denom_2"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_2", "denom_1"), &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_3", "denom_1"), &Uint128::new(100)).unwrap();

    let res = paginate_prefix_query(
        &coin_balances,
        &deps.storage,
        "account_x",
        None,
        3,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 0);
}

#[test]
pub fn has_more_false_when_all_prefixes_within_limit() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<(&str, &str), Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, ("account_1", "denom_1"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_1", "denom_2"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_2", "denom_1"), &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_3", "denom_1"), &Uint128::new(100)).unwrap();

    let res = paginate_prefix_query(
        &coin_balances,
        &deps.storage,
        "account_1",
        None,
        3,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 2);
}

#[test]
pub fn has_more_true_when_results_outside_limit() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<(&str, &str), Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, ("account_1", "denom_1"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_1", "denom_2"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_2", "denom_1"), &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_3", "denom_1"), &Uint128::new(100)).unwrap();

    let res = paginate_prefix_query(
        &coin_balances,
        &deps.storage,
        "account_1",
        None,
        1,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(res.metadata.has_more);
    assert_eq!(res.data.len(), 1);
}

#[test]
pub fn empty_when_start_after_not_found() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<(&str, &str), Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, ("account_1", "denom_1"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_1", "denom_2"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_2", "denom_1"), &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_3", "denom_1"), &Uint128::new(100)).unwrap();

    let res = paginate_prefix_query(
        &coin_balances,
        &deps.storage,
        "account_1",
        Some(Bound::inclusive("denom_x")),
        1,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 0);
}

#[test]
pub fn has_more_false_when_start_is_last_alphabetically() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<(&str, &str), Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, ("account_1", "denom_1"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_1", "denom_2"), &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_2", "denom_1"), &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, ("account_3", "denom_1"), &Uint128::new(100)).unwrap();

    let res = paginate_prefix_query(
        &coin_balances,
        &deps.storage,
        "account_1",
        Some(Bound::inclusive("denom_2")),
        1,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 1);
}
