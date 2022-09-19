use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coin, Addr, Api};
use itertools::Itertools;

use rover::adapters::VaultBase;
use rover::msg::execute::Action;

use crate::helpers::{
    assert_contents_equal, build_mock_vaults, uatom_info, uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn test_pagination_on_all_vault_positions_query_works() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let all_vaults = build_mock_vaults(22);
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: vec![
                coin(1000, uosmo.denom.clone()),
                coin(1000, uatom.denom.clone()),
            ],
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: vec![
                coin(1000, uosmo.denom.clone()),
                coin(1000, uatom.denom.clone()),
            ],
        })
        .fund_account(AccountToFund {
            addr: user_c.clone(),
            funds: vec![
                coin(1000, uosmo.denom.clone()),
                coin(1000, uatom.denom.clone()),
            ],
        })
        .allowed_coins(&[uosmo.clone(), uatom.clone()])
        .allowed_vaults(&all_vaults)
        .build()
        .unwrap();

    let mut actions = vec![
        Action::Deposit(uatom.to_coin(220)),
        Action::Deposit(uosmo.to_coin(220)),
    ];

    all_vaults.iter().for_each(|v| {
        actions.extend([Action::VaultDeposit {
            vault: mock.get_vault(v),
            coins: vec![uatom.to_coin(10), uosmo.to_coin(10)],
        }]);
    });

    let account_id_a = mock.create_credit_account(&user_a).unwrap();
    mock.update_credit_account(
        &account_id_a,
        &user_a,
        actions.clone(),
        &[uatom.to_coin(220), uosmo.to_coin(220)],
    )
    .unwrap();

    let account_id_b = mock.create_credit_account(&user_b).unwrap();
    mock.update_credit_account(
        &account_id_b,
        &user_b,
        actions.clone(),
        &[uatom.to_coin(220), uosmo.to_coin(220)],
    )
    .unwrap();

    let account_id_c = mock.create_credit_account(&user_c).unwrap();
    mock.update_credit_account(
        &account_id_c,
        &user_c,
        actions,
        &[uatom.to_coin(220), uosmo.to_coin(220)],
    )
    .unwrap();

    let vaults_res = mock.query_all_vault_positions(None, Some(58_u32));
    // Assert maximum is observed
    assert_eq!(vaults_res.len(), 30);

    let vaults_res = mock.query_all_vault_positions(None, Some(2_u32));
    // Assert limit request is observed
    assert_eq!(vaults_res.len(), 2);

    let vaults_res_a = mock.query_all_vault_positions(None, None);
    let item = vaults_res_a.last().unwrap();
    let vaults_res_b = mock
        .query_all_vault_positions(Some((item.account_id.clone(), item.addr.clone())), Some(30));
    let item = vaults_res_b.last().unwrap();
    let vaults_res_c = mock
        .query_all_vault_positions(Some((item.account_id.clone(), item.addr.clone())), Some(30));
    let item = vaults_res_c.last().unwrap();
    let vaults_res_d =
        mock.query_all_vault_positions(Some((item.account_id.clone(), item.addr.clone())), None);

    // Assert default is observed
    assert_eq!(vaults_res_a.len(), 10);
    assert_eq!(vaults_res_b.len(), 30);
    assert_eq!(vaults_res_c.len(), 26);
    assert_eq!(vaults_res_d.len(), 0);

    let combined = vaults_res_a
        .iter()
        .cloned()
        .chain(vaults_res_b.iter().cloned())
        .chain(vaults_res_c.iter().cloned())
        .chain(vaults_res_d.iter().cloned())
        .map(|v| VaultBase::new(MockApi::default().addr_validate(v.addr.as_str()).unwrap()))
        .map(|v| v.query_vault_info(&mock.app.wrap()).unwrap())
        .map(|info| info.token_denom)
        .collect::<Vec<_>>();

    let deduped = combined.iter().unique().cloned().collect::<Vec<_>>();

    assert_eq!(deduped.len(), all_vaults.len());

    assert_contents_equal(
        all_vaults
            .iter()
            .map(|v| v.lp_token_denom.clone())
            .collect(),
        deduped,
    )
}
