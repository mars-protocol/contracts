use cosmwasm_std::{Addr, StdError};
use mars_account_nft::error::{ContractError, ContractError::HealthContractNotSet};
use mars_account_nft_types::msg::QueryMsg::{AllTokens, NumTokens, Tokens};
use mars_rover_health_types::AccountKind;

use crate::helpers::{generate_health_response, MockEnv};

pub mod helpers;

#[test]
fn burning_empty_accounts_not_allowed_if_no_health_contract_set() {
    let mut mock = MockEnv::new().instantiate_with_health_contract(false).build().unwrap();
    let user = Addr::unchecked("user");
    mock.mint(&user).unwrap();
    let res = mock.burn_empty_accounts(&user, None);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(error, HealthContractNotSet);
}

#[test]
fn burn_empty_accounts() {
    let mut mock = MockEnv::new().build().unwrap();

    // create few accounts
    let user_1 = Addr::unchecked("user_1");
    let user_1_token_id = mock.mint(&user_1).unwrap();
    mock.set_health_response(
        &user_1,
        &user_1_token_id,
        AccountKind::Default,
        &generate_health_response(10_000, 0),
    );
    let user_2 = Addr::unchecked("user_2");
    let user_2_token_id_1 = mock.mint(&user_2).unwrap();
    mock.set_health_response(
        &user_2,
        &user_2_token_id_1,
        AccountKind::Default,
        &generate_health_response(0, 0),
    );
    let user_2_token_id_2 = mock.mint(&user_2).unwrap();
    mock.set_health_response(
        &user_2,
        &user_2_token_id_2,
        AccountKind::Default,
        &generate_health_response(0, 1),
    );
    let user_3 = Addr::unchecked("user_3");
    let user_3_token_id = mock.mint(&user_3).unwrap();
    mock.set_health_response(
        &user_3,
        &user_3_token_id,
        AccountKind::Default,
        &generate_health_response(1, 1),
    );
    let user_4 = Addr::unchecked("user_4");
    let user_4_token_id = mock.mint(&user_4).unwrap();
    mock.set_health_response(
        &user_4,
        &user_4_token_id,
        AccountKind::Default,
        &generate_health_response(0, 0),
    );
    let user_5 = Addr::unchecked("user_5");
    let user_5_token_id = mock.mint(&user_5).unwrap();
    mock.set_health_response(
        &user_5,
        &user_5_token_id,
        AccountKind::Default,
        &generate_health_response(0, 0),
    );
    let user_6 = Addr::unchecked("user_6");
    let user_6_token_id = mock.mint(&user_6).unwrap();
    mock.set_health_response(
        &user_6,
        &user_6_token_id,
        AccountKind::Default,
        &generate_health_response(0, 100),
    );

    // check that all accounts are created
    let res: cw721::NumTokensResponse =
        mock.app.wrap().query_wasm_smart(mock.nft_contract.clone(), &NumTokens {}).unwrap();
    assert_eq!(res.count, 7);

    // check that for user 2 there are 2 tokens
    let res: cw721::TokensResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract.clone(),
            &Tokens {
                owner: user_2.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res.tokens, vec![user_2_token_id_1.clone(), user_2_token_id_2.clone()]);

    // burn empty accounts
    let user = Addr::unchecked("random_user");
    mock.burn_empty_accounts(&user, Some(2)).unwrap();
    mock.burn_empty_accounts(&user, Some(2)).unwrap();
    mock.burn_empty_accounts(&user, Some(2)).unwrap();
    mock.burn_empty_accounts(&user, Some(2)).unwrap();
    mock.burn_empty_accounts(&user, Some(2)).unwrap(); // set flag to Finished
    let res = mock.burn_empty_accounts(&user, Some(2));
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        ContractError::Std(StdError::generic_err(
            "Migration completed. All empty accounts already burned."
        ))
    );

    // check that only empty accounts are burned
    let res: cw721::TokensResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract.clone(),
            &AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        res.tokens,
        vec![user_1_token_id, user_2_token_id_2.clone(), user_3_token_id, user_6_token_id]
    );

    // check that for user 2 there is only one token, second one should be burned
    let res: cw721::TokensResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract.clone(),
            &Tokens {
                owner: user_2.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res.tokens, vec![user_2_token_id_2]);
}
