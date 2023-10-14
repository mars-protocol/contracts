use cosmwasm_std::{Addr, Empty};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;

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
