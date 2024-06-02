use cosmwasm_std::{coin, Addr, Empty};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;
use mars_types::health::AccountKind;
use test_case::test_case;

use super::helpers::MockEnv;
use crate::tests::helpers::{deploy_managed_vault, AccountToFund};

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

#[test_case("Mars_default", AccountKind::Default; "create Default account")]
#[test_case("Mars_HLS", AccountKind::HighLeveredStrategy; "create HLS account")]
fn create_credit_account_v2(custom_account_id: &str, account_kind: AccountKind) {
    let mut mock = MockEnv::new().build().unwrap();

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let user = Addr::unchecked("user_123");
    let account_id = mock
        .create_credit_account_v2(&user, account_kind.clone(), Some(custom_account_id.to_string()))
        .unwrap();
    assert_eq!(account_id, custom_account_id.to_string());

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.clone().unwrap(),
            &NftQueryMsg::<Empty>::OwnerOf {
                token_id: account_id.clone(),
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user, owner_res.owner);
    let position = mock.query_positions(&account_id);
    assert_eq!(position.account_kind, account_kind);
}

#[test]
fn create_fund_manager_credit_account_v2() {
    let custom_account_id = "Mars_Vault";
    let user = Addr::unchecked("user_123");

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .build()
        .unwrap();

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let credit_manager = mock.rover.clone();
    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &user, &credit_manager);

    let account_kind = AccountKind::FundManager {
        vault_addr: managed_vault_addr.to_string(),
    };
    let account_id = mock
        .create_credit_account_v2(&user, account_kind.clone(), Some(custom_account_id.to_string()))
        .unwrap();
    assert_eq!(account_id, custom_account_id.to_string());

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.clone().unwrap(),
            &NftQueryMsg::<Empty>::OwnerOf {
                token_id: account_id.clone(),
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user, owner_res.owner);
    let position = mock.query_positions(&account_id);
    assert_eq!(position.account_kind, account_kind);
}
