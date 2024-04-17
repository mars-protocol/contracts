use cosmwasm_std::{testing::mock_dependencies, StdError, Uint128};
use cw_storage_plus::{Bound, Map};
use mars_types::paginate_map_query;

#[test]
pub fn empty_when_start_not_found() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<&str, Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, "key_1", &Uint128::new(100)).unwrap();
    coin_balances.save(&mut deps.storage, "key_2", &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, "key_3", &Uint128::new(102)).unwrap();

    let res = paginate_map_query(
        &coin_balances,
        &deps.storage,
        Some(Bound::exclusive("key_x")),
        3,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 0);
}

#[test]
pub fn has_more_true_when_limit_not_reached() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<&str, Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, "key_1", &Uint128::new(100)).unwrap();
    coin_balances.save(&mut deps.storage, "key_2", &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, "key_3", &Uint128::new(102)).unwrap();

    let res = paginate_map_query(&coin_balances, &deps.storage, None, 2, |_key, amount| {
        Ok::<Uint128, StdError>(amount)
    })
    .unwrap();

    assert!(res.metadata.has_more);
    assert_eq!(res.data.len(), 2);
}

#[test]
pub fn has_more_false_when_limit_reached() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<&str, Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, "key_1", &Uint128::new(100)).unwrap();
    coin_balances.save(&mut deps.storage, "key_2", &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, "key_3", &Uint128::new(102)).unwrap();

    let res = paginate_map_query(&coin_balances, &deps.storage, None, 3, |_key, amount| {
        Ok::<Uint128, StdError>(amount)
    })
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 3);
}

#[test]
pub fn empty_when_map_is_empty() {
    let deps = mock_dependencies();

    let coin_balances: Map<&str, Uint128> = Map::new("coin_balance");

    let res = paginate_map_query(&coin_balances, &deps.storage, None, 3, |_key, amount| {
        Ok::<Uint128, StdError>(amount)
    })
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 0);
}

#[test]
pub fn has_more_false_when_start_is_last_alphabetically() {
    let mut deps = mock_dependencies();

    let coin_balances: Map<&str, Uint128> = Map::new("coin_balance");

    coin_balances.save(&mut deps.storage, "key_3", &Uint128::new(102)).unwrap();
    coin_balances.save(&mut deps.storage, "key_2", &Uint128::new(101)).unwrap();
    coin_balances.save(&mut deps.storage, "key_1", &Uint128::new(100)).unwrap();

    let res = paginate_map_query(
        &coin_balances,
        &deps.storage,
        Some(Bound::inclusive("key_3")),
        3,
        |_key, amount| Ok::<Uint128, StdError>(amount),
    )
    .unwrap();

    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), 1);
}
