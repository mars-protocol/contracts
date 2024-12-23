use cosmwasm_std::{
    attr, from_json,
    testing::{mock_env, mock_info},
    Empty, Event,
};
use cw2::{ContractVersion, VersionError};
use cw721_base::InstantiateMsg;
use mars_account_nft::{
    contract::{execute, migrate, query, Parent},
    error::ContractError,
    state::NEXT_ID,
};
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-account-nft".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-account-nft", "4.1.0")
        .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-account-nft", "2.0.0")
        .unwrap();

    let env = mock_env();
    // Credit-Manager contract address
    let owner = "osmo1f2m24wktq0sw3c0lexlg7fv4kngwyttvzws3a3r3al9ld2s2pvds87jqvf";
    let owner_info = mock_info(owner, &[]);

    // Init counter with 1
    NEXT_ID.save(deps.as_mut().storage, &1).unwrap();

    // Instantiate the contract
    Parent::default()
        .instantiate(
            deps.as_mut(),
            env.clone(),
            owner_info.clone(),
            InstantiateMsg {
                name: "mock_nft".to_string(),
                symbol: "MOCK".to_string(),
                minter: owner.to_string(),
            },
        )
        .unwrap();

    // Mint a random token
    execute(
        deps.as_mut(),
        env.clone(),
        owner_info.clone(),
        mars_types::account_nft::ExecuteMsg::Mint {
            user: "user_1".to_string(),
        },
    )
    .unwrap();

    // Move counter to 3000
    NEXT_ID.save(deps.as_mut().storage, &3000).unwrap();

    // Mint a random token
    execute(
        deps.as_mut(),
        env.clone(),
        owner_info.clone(),
        mars_types::account_nft::ExecuteMsg::Mint {
            user: "user_2".to_string(),
        },
    )
    .unwrap();

    // Check if counter is moved to 3001
    let next_id = NEXT_ID.load(deps.as_ref().storage).unwrap();
    assert_eq!(next_id, 3001);

    // Query should fail because token_id 2321 is not minted yet
    query(
        deps.as_ref(),
        env.clone(),
        mars_types::account_nft::QueryMsg::OwnerOf {
            token_id: "2321".to_string(),
            include_expired: None,
        },
    )
    .unwrap_err();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    // Query should now return the owner of token_id 2321, which is Rewards-Collector contract address
    let owner_of_res_binary = query(
        deps.as_ref(),
        env.clone(),
        mars_types::account_nft::QueryMsg::OwnerOf {
            token_id: "2321".to_string(),
            include_expired: None,
        },
    )
    .unwrap();
    let owner_of_res: cw721::OwnerOfResponse = from_json(owner_of_res_binary).unwrap();
    assert_eq!(
        owner_of_res.owner,
        "osmo1urvqe5mw00ws25yqdd4c4hlh8kdyf567mpcml7cdve9w08z0ydcqvsrgdy".to_string()
    );

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.1.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-account-nft".to_string(),
        version: "2.1.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
