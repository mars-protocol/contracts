use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, from_binary, Addr};

use fields::messages::{AllowListsResponse, InstantiateMsg, OwnerResponse, QueryMsg};
use fields::types::AssetInfo;

use crate::contract::{instantiate, query};

#[test]
fn test_owner_set_on_instantiate() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &coins(1000, "uosmo"));

    let owner_str = String::from("spiderman123");
    let res = instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            owner: owner_str.clone(),
            allowed_vaults: vec![],
            allowed_assets: vec![],
        },
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner {}).unwrap();
    let value: OwnerResponse = from_binary(&res).unwrap();
    assert_eq!(owner_str, value.owner);
}

#[test]
fn test_allowed_vaults_and_assets_stored_on_instantiate() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &coins(1000, "uosmo"));

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

    instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            owner: String::from("spiderman123"),
            allowed_vaults: allowed_vaults.clone(),
            allowed_assets: allowed_assets.clone(),
        },
    )
    .unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetAllowLists {}).unwrap();
    let res: AllowListsResponse = from_binary(&res).unwrap();
    assert_eq!(res.vaults.len(), 3);
    assert_eq!(allowed_vaults, res.vaults);

    assert_eq!(res.assets.len(), 4);
    assert!(allowed_assets.iter().all(|item| res.assets.contains(item)));
}

#[test]
fn test_panics_on_invalid_instantiation_addrs() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &coins(1000, "uosmo"));

    let res = instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg {
            owner: String::from("spiderman123"),
            allowed_vaults: vec![String::from("123INVALID")],
            allowed_assets: vec![],
        },
    );

    match res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }

    let res = instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            owner: String::from("spiderman123"),
            allowed_vaults: vec![],
            allowed_assets: vec![AssetInfo::Cw20(Addr::unchecked("123INVALID"))],
        },
    );

    match res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }
}
