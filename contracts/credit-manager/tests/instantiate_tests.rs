use cosmwasm_std::Addr;
use cw_asset::AssetInfo;
use cw_multi_test::Executor;

use fields::messages::{AllowListsResponse, InstantiateMsg, OwnerResponse, QueryMsg};

use crate::helpers::{mock_app, mock_contract};

mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
    };

    let contract_addr =
        app.instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-contract", None).unwrap();

    let res: OwnerResponse =
        app.wrap().query_wasm_smart(contract_addr.clone(), &QueryMsg::GetOwner {}).unwrap();

    assert_eq!(owner, res.owner);
}

#[test]
fn test_allowed_vaults_and_assets_stored_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_vaults = vec![
        String::from("vaultcontract1"),
        String::from("vaultcontract2"),
        String::from("vaultcontract3"),
    ];

    let allowed_assets = vec![
        AssetInfo::Native(String::from("uosmo")),
        AssetInfo::Cw20(Addr::unchecked("osmo85wwjycfxjlaxsae9asmxlk3bsgxbw")),
        AssetInfo::Cw20(Addr::unchecked("osmompbtkt3jezatztteo577lxkqbkdyke")),
        AssetInfo::Cw20(Addr::unchecked("osmos6kmpxz9xcstleqnu2fnz8gskgf6gx")),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: allowed_vaults.clone(),
        allowed_assets: allowed_assets.clone(),
    };

    let contract_addr =
        app.instantiate_contract(code_id, owner, &msg, &[], "mock-contract", None).unwrap();

    let res: AllowListsResponse =
        app.wrap().query_wasm_smart(contract_addr.clone(), &QueryMsg::GetAllowLists {}).unwrap();

    assert_eq!(res.vaults.len(), 3);
    assert_eq!(allowed_vaults, res.vaults);

    assert_eq!(res.assets.len(), 4);
    assert!(allowed_assets.iter().all(|item| res.assets.contains(item)));
}

#[test]
fn test_panics_on_invalid_instantiation_addrs() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![String::from("123INVALID")],
        allowed_assets: vec![],
    };

    let instantiate_res =
        app.instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-contract", None);

    match instantiate_res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![AssetInfo::Cw20(Addr::unchecked("123INVALID"))],
    };

    let instantiate_res =
        app.instantiate_contract(code_id, owner, &msg, &[], "mock-contract", None);

    match instantiate_res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }
}
