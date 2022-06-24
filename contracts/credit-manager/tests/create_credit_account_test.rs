use cosmwasm_std::Addr;
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;
use cw_multi_test::Executor;

use rover::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{mock_account_nft_contract, mock_app, mock_contract};

mod helpers;

#[test]
fn test_create_credit_account() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");

    let nft_contract_code_id = app.store_code(mock_account_nft_contract());

    let credit_manager_code_id = app.store_code(mock_contract());
    let manager_initiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
        nft_contract_code_id,
    };

    let contract_addr = app
        .instantiate_contract(
            credit_manager_code_id,
            owner.clone(),
            &manager_initiate_msg,
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let user = Addr::unchecked("some_user");
    let res = app
        .execute_contract(
            user.clone(),
            contract_addr.clone(),
            &ExecuteMsg::CreateCreditAccount {},
            &[],
        )
        .unwrap();

    let attr: Vec<&String> = res
        .events
        .iter()
        .flat_map(|event| &event.attributes)
        .filter(|attr| attr.key == "token_id")
        .map(|attr| &attr.value)
        .collect();

    assert_eq!(attr.len(), 1);
    let token_id = attr.first().unwrap().as_str();
    assert_eq!(token_id, "1");

    // Double checking ownership by querying NFT account-nft for correct owner
    let nft_contract_res: String = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::CreditAccountNftAddress {})
        .unwrap();

    let owner_res: OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            nft_contract_res,
            &NftQueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user, owner_res.owner)
}
