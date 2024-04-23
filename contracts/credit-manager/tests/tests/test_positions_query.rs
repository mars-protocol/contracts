use cosmwasm_std::Addr;
use mars_types::health::AccountKind;

use crate::tests::helpers::MockEnv;

#[test]
fn returns_account_kind() {
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id_1 = mock.create_credit_account(&user1).unwrap();
    let account_id_2 = mock.create_hls_account(&user2);

    let position_1 = mock.query_positions(&account_id_1);
    let position_2 = mock.query_positions(&account_id_2);

    assert_eq!(position_1.kind, AccountKind::Default);
    assert_eq!(position_2.kind, AccountKind::HighLeveredStrategy);
}
