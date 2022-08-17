use crate::helpers::MockEnv;
use cosmwasm_std::Addr;
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;

pub mod helpers;

#[test]
fn test_create_credit_account_fails_without_nft_contract_set() {
    let mut mock = MockEnv::new().no_nft_contract().build().unwrap();
    let user = Addr::unchecked("user");
    let res = mock.create_credit_account(&user);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not yet set");
    }
}

#[test]
fn test_create_credit_account_fails_without_nft_contract_owner() {
    let mut mock = MockEnv::new().no_nft_contract_owner().build().unwrap();

    let user = Addr::unchecked("user");
    let res = mock.create_credit_account(&user);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not proposing a new owner yet");
    }
}

#[test]
fn test_create_credit_account_success() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.create_credit_account(&user).unwrap();

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.unwrap(),
            &NftQueryMsg::OwnerOf {
                token_id,
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user, owner_res.owner)
}
