use cosmwasm_std::Addr;
use mars_rover::msg::query::Account;
use mars_rover_health_types::AccountKind;

use crate::helpers::MockEnv;

pub mod helpers;

fn account_default(id: &str) -> Account {
    Account {
        id: id.to_string(),
        kind: AccountKind::Default,
    }
}

fn account_hls(id: &str) -> Account {
    Account {
        id: id.to_string(),
        kind: AccountKind::HighLeveredStrategy,
    }
}

#[test]
fn pagination_on_accounts_query_works() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let mut mock = MockEnv::new().build().unwrap();

    let account_id_a_2 = mock.create_credit_account(&user_a).unwrap();
    assert_eq!(account_id_a_2, "2".to_string()); // assert starting number
    mock.create_credit_account(&user_a).unwrap();
    mock.create_credit_account(&user_b).unwrap();
    mock.create_hls_account(&user_a);
    mock.create_credit_account(&user_c).unwrap();
    mock.create_credit_account(&user_a).unwrap();
    mock.create_credit_account(&user_a).unwrap();
    mock.create_hls_account(&user_b);
    mock.create_hls_account(&user_b);
    mock.create_credit_account(&user_b).unwrap();
    mock.create_hls_account(&user_c);

    let user_a_accounts = mock.query_accounts(user_a.as_str(), None, Some(2));
    assert_eq!(user_a_accounts, vec![account_default("2"), account_default("3")]);

    let user_a_accounts = mock.query_accounts(user_a.as_str(), Some("3".to_string()), Some(2));
    assert_eq!(user_a_accounts, vec![account_hls("5"), account_default("7")]);

    let user_a_accounts = mock.query_accounts(user_a.as_str(), Some("7".to_string()), Some(2));
    assert_eq!(user_a_accounts, vec![account_default("8")]);

    let user_b_accounts = mock.query_accounts(user_b.as_str(), None, None);
    assert_eq!(
        user_b_accounts,
        vec![account_hls("10"), account_default("11"), account_default("4"), account_hls("9")]
    );

    let user_c_accounts = mock.query_accounts(user_c.as_str(), None, None);
    assert_eq!(user_c_accounts, vec![account_hls("12"), account_default("6")]);

    let user_c_accounts = mock.query_accounts(user_c.as_str(), Some("6".to_string()), None);
    assert_eq!(user_c_accounts, vec![]);
}
