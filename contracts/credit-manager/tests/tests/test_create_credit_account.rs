use cosmwasm_std::{Addr, Empty};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;
use mars_types::health::AccountKind;

use super::helpers::MockEnv;

#[test]
fn create_credit_account_fails_without_nft_contract_set() {
    let mut mock = MockEnv::new().no_nft_contract().build().unwrap();
    let user = Addr::unchecked("user");
    let res = mock.create_credit_account(&user);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not yet set");
    }
}

#[test]
fn create_credit_account_fails_without_nft_contract_owner() {
    let mut mock = MockEnv::new().no_nft_contract_minter().build().unwrap();

    let user = Addr::unchecked("user");
    let res = mock.create_credit_account(&user);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not proposing a new owner yet");
    }
}

#[test]
fn create_credit_account_success() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.unwrap(),
            &NftQueryMsg::<Empty>::OwnerOf {
                token_id: account_id,
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user, owner_res.owner)
}

#[test]
fn after_create_returns_account_kind() {
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id_1 = mock.create_credit_account(&user1).unwrap();
    let account_id_2 = mock.create_hls_account(&user2);

    let position_1 = mock.query_positions(&account_id_1);
    let position_2 = mock.query_positions(&account_id_2);

    assert_eq!(position_1.account_kind, AccountKind::Default);
    assert_eq!(position_2.account_kind, AccountKind::HighLeveredStrategy);
}
