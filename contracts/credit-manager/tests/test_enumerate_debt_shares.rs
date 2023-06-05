use cosmwasm_std::{coin, Addr};
use mars_credit_manager::borrow::DEFAULT_DEBT_SHARES_PER_COIN_BORROWED;
use mars_rover::msg::{execute::Action, query::SharesResponseItem};

use crate::helpers::{build_mock_coin_infos, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn pagination_on_all_debt_shares_query_works() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let user_a_coins = vec![
        coin(10, "coin_1"),
        coin(10, "coin_2"),
        coin(10, "coin_3"),
        coin(10, "coin_4"),
        coin(10, "coin_5"),
        coin(10, "coin_6"),
        coin(10, "coin_7"),
        coin(10, "coin_8"),
        coin(10, "coin_9"),
        coin(10, "coin_10"),
        coin(10, "coin_11"),
        coin(10, "coin_12"),
        coin(10, "coin_13"),
        coin(10, "coin_14"),
    ];

    let user_b_coins = vec![
        coin(10, "coin_15"),
        coin(10, "coin_16"),
        coin(10, "coin_17"),
        coin(10, "coin_18"),
        coin(10, "coin_19"),
        coin(10, "coin_20"),
        coin(10, "coin_21"),
        coin(10, "coin_22"),
        coin(10, "coin_23"),
        coin(10, "coin_24"),
    ];

    let user_c_coins = vec![
        coin(10, "coin_25"),
        coin(10, "coin_26"),
        coin(10, "coin_27"),
        coin(10, "coin_28"),
        coin(10, "coin_29"),
        coin(10, "coin_30"),
        coin(10, "coin_31"),
        coin(10, "coin_32"),
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
        .set_params(&build_mock_coin_infos(32))
        .build()
        .unwrap();

    let account_id_a = mock.create_credit_account(&user_a).unwrap();
    mock.update_credit_account(
        &account_id_a,
        &user_a,
        user_a_coins
            .iter()
            .flat_map(|c| {
                vec![Action::Deposit(c.clone()), Action::Borrow(coin(1, c.denom.clone()))]
            })
            .collect::<Vec<Action>>(),
        &user_a_coins,
    )
    .unwrap();

    let account_id_b = mock.create_credit_account(&user_b).unwrap();
    mock.update_credit_account(
        &account_id_b,
        &user_b,
        user_b_coins
            .iter()
            .flat_map(|c| {
                vec![Action::Deposit(c.clone()), Action::Borrow(coin(1, c.denom.clone()))]
            })
            .collect::<Vec<Action>>(),
        &user_b_coins,
    )
    .unwrap();

    let account_id_c = mock.create_credit_account(&user_c).unwrap();
    mock.update_credit_account(
        &account_id_c,
        &user_c,
        user_c_coins
            .iter()
            .flat_map(|c| {
                vec![Action::Deposit(c.clone()), Action::Borrow(coin(1, c.denom.clone()))]
            })
            .collect::<Vec<Action>>(),
        &user_c_coins,
    )
    .unwrap();

    let all_debt_shares_res = mock.query_all_debt_shares(None, Some(58_u32));

    // Assert maximum is observed
    assert_eq!(all_debt_shares_res.len(), 30);

    let all_debt_shares_res = mock.query_all_debt_shares(None, Some(2_u32));

    // Assert limit request is observed
    assert_eq!(all_debt_shares_res.len(), 2);

    let all_debt_shares_res_a = mock.query_all_debt_shares(None, None);

    let SharesResponseItem {
        account_id,
        denom,
        ..
    } = all_debt_shares_res_a.last().unwrap().clone();
    let all_debt_shares_res_b = mock.query_all_debt_shares(Some((account_id, denom)), None);

    let SharesResponseItem {
        account_id,
        denom,
        ..
    } = all_debt_shares_res_b.last().unwrap().clone();
    let all_debt_shares_res_c = mock.query_all_debt_shares(Some((account_id, denom)), None);

    let SharesResponseItem {
        account_id,
        denom,
        ..
    } = all_debt_shares_res_c.last().unwrap().clone();
    let all_debt_shares_res_d = mock.query_all_debt_shares(Some((account_id, denom)), None);

    // Assert default is observed
    assert_eq!(all_debt_shares_res_a.len(), 10);
    assert_eq!(all_debt_shares_res_b.len(), 10);
    assert_eq!(all_debt_shares_res_c.len(), 10);

    assert_eq!(all_debt_shares_res_d.len(), 2);

    let combined_res: Vec<SharesResponseItem> = all_debt_shares_res_a
        .iter()
        .cloned()
        .chain(all_debt_shares_res_b.iter().cloned())
        .chain(all_debt_shares_res_c.iter().cloned())
        .chain(all_debt_shares_res_d.iter().cloned())
        .collect();

    let user_a_response_items = user_a_coins
        .iter()
        .map(|coin| SharesResponseItem {
            account_id: account_id_a.clone(),
            denom: coin.denom.clone(),
            shares: DEFAULT_DEBT_SHARES_PER_COIN_BORROWED,
        })
        .collect::<Vec<SharesResponseItem>>();

    let user_b_response_items = user_b_coins
        .iter()
        .map(|coin| SharesResponseItem {
            account_id: account_id_b.clone(),
            denom: coin.denom.clone(),
            shares: DEFAULT_DEBT_SHARES_PER_COIN_BORROWED,
        })
        .collect::<Vec<SharesResponseItem>>();

    let user_c_response_items = user_c_coins
        .iter()
        .map(|coin| SharesResponseItem {
            account_id: account_id_c.clone(),
            denom: coin.denom.clone(),
            shares: DEFAULT_DEBT_SHARES_PER_COIN_BORROWED,
        })
        .collect::<Vec<SharesResponseItem>>();

    let combined_starting_vals: Vec<SharesResponseItem> = user_a_response_items
        .iter()
        .cloned()
        .chain(user_b_response_items)
        .chain(user_c_response_items)
        .collect();

    assert_eq!(combined_res.len(), combined_starting_vals.len());
    assert!(combined_starting_vals.iter().all(|item| combined_res.contains(item)));
}
