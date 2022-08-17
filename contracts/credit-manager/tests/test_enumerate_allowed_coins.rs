use crate::helpers::{build_mock_coin_infos, MockEnv};

pub mod helpers;

#[test]
fn test_pagination_on_allowed_coins_query_works() {
    let allowed_coins = build_mock_coin_infos(32);
    let mock = MockEnv::new()
        .allowed_coins(&build_mock_coin_infos(32))
        .build()
        .unwrap();

    let coins_res = mock.query_allowed_coins(None, Some(58_u32));

    // Assert maximum is observed
    assert_eq!(coins_res.len(), 30);

    let coins_res = mock.query_allowed_coins(None, Some(2_u32));

    // Assert limit request is observed
    assert_eq!(coins_res.len(), 2);

    let coins_res_a = mock.query_allowed_coins(None, None);
    let coins_res_b = mock.query_allowed_coins(Some(coins_res_a.last().unwrap().clone()), None);
    let coins_res_c = mock.query_allowed_coins(Some(coins_res_b.last().unwrap().clone()), None);
    let coins_res_d = mock.query_allowed_coins(Some(coins_res_c.last().unwrap().clone()), None);

    // Assert default is observed
    assert_eq!(coins_res_a.len(), 10);
    assert_eq!(coins_res_b.len(), 10);
    assert_eq!(coins_res_c.len(), 10);

    assert_eq!(coins_res_d.len(), 2);

    let combined: Vec<String> = coins_res_a
        .iter()
        .cloned()
        .chain(coins_res_b.iter().cloned())
        .chain(coins_res_c.iter().cloned())
        .chain(coins_res_d.iter().cloned())
        .collect();

    assert_eq!(combined.len(), allowed_coins.len());
    assert!(allowed_coins
        .iter()
        .all(|item| combined.contains(&item.denom)));
}
