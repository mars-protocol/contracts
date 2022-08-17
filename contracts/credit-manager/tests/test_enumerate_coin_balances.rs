use cosmwasm_std::{Addr, Coin, Uint128};

use rover::msg::execute::Action;
use rover::msg::query::CoinBalanceResponseItem;

use crate::helpers::{build_mock_coin_infos, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn test_pagination_on_all_coin_balances_query_works() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let user_a_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
        Coin::new(1u128, "coin_9"),
        Coin::new(1u128, "coin_10"),
        Coin::new(1u128, "coin_11"),
        Coin::new(1u128, "coin_12"),
        Coin::new(1u128, "coin_13"),
        Coin::new(1u128, "coin_14"),
    ];

    let user_b_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
        Coin::new(1u128, "coin_9"),
        Coin::new(1u128, "coin_10"),
    ];

    let user_c_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
    ];

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: user_a_coins.clone(),
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: user_b_coins.clone(),
        })
        .fund_account(AccountToFund {
            addr: user_c.clone(),
            funds: user_c_coins.clone(),
        })
        .allowed_coins(&build_mock_coin_infos(14))
        .build()
        .unwrap();

    let token_id_a = mock.create_credit_account(&user_a).unwrap();
    mock.update_credit_account(
        &token_id_a,
        &user_a,
        user_a_coins
            .iter()
            .map(|coin| Action::Deposit(coin.clone()))
            .collect(),
        &user_a_coins,
    )
    .unwrap();

    let token_id_b = mock.create_credit_account(&user_b).unwrap();
    mock.update_credit_account(
        &token_id_b,
        &user_b,
        user_b_coins
            .iter()
            .map(|coin| Action::Deposit(coin.clone()))
            .collect(),
        &user_b_coins,
    )
    .unwrap();

    let token_id_c = mock.create_credit_account(&user_c).unwrap();
    mock.update_credit_account(
        &token_id_c,
        &user_c,
        user_c_coins
            .iter()
            .map(|coin| Action::Deposit(coin.clone()))
            .collect(),
        &user_c_coins,
    )
    .unwrap();

    let all_assets_res = mock.query_all_coin_balances(None, Some(58_u32));

    // Assert maximum is observed
    assert_eq!(all_assets_res.len(), 30);

    let all_assets_res = mock.query_all_coin_balances(None, Some(2_u32));

    // Assert limit request is observed
    assert_eq!(all_assets_res.len(), 2);

    let all_assets_res_a = mock.query_all_coin_balances(None, None);

    let CoinBalanceResponseItem {
        token_id, denom, ..
    } = all_assets_res_a.last().unwrap().clone();
    let all_assets_res_b = mock.query_all_coin_balances(Some((token_id, denom)), None);

    let CoinBalanceResponseItem {
        token_id, denom, ..
    } = all_assets_res_b.last().unwrap().clone();
    let all_assets_res_c = mock.query_all_coin_balances(Some((token_id, denom)), None);

    let CoinBalanceResponseItem {
        token_id, denom, ..
    } = all_assets_res_c.last().unwrap().clone();
    let all_assets_res_d = mock.query_all_coin_balances(Some((token_id, denom)), None);

    // Assert default is observed
    assert_eq!(all_assets_res_a.len(), 10);
    assert_eq!(all_assets_res_b.len(), 10);
    assert_eq!(all_assets_res_c.len(), 10);

    assert_eq!(all_assets_res_d.len(), 2);

    let combined_res: Vec<CoinBalanceResponseItem> = all_assets_res_a
        .iter()
        .cloned()
        .chain(all_assets_res_b.iter().cloned())
        .chain(all_assets_res_c.iter().cloned())
        .chain(all_assets_res_d.iter().cloned())
        .collect();

    let user_a_response_items = user_a_coins
        .iter()
        .map(|coin| CoinBalanceResponseItem {
            token_id: token_id_a.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<CoinBalanceResponseItem>>();

    let user_b_response_items = user_b_coins
        .iter()
        .map(|coin| CoinBalanceResponseItem {
            token_id: token_id_b.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<CoinBalanceResponseItem>>();

    let user_c_response_items = user_c_coins
        .iter()
        .map(|coin| CoinBalanceResponseItem {
            token_id: token_id_c.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<CoinBalanceResponseItem>>();

    let combined_starting_vals: Vec<CoinBalanceResponseItem> = user_a_response_items
        .iter()
        .cloned()
        .chain(user_b_response_items)
        .chain(user_c_response_items)
        .collect();

    assert_eq!(combined_res.len(), combined_starting_vals.len());
    assert!(combined_starting_vals
        .iter()
        .all(|item| combined_res.contains(item)));
}
