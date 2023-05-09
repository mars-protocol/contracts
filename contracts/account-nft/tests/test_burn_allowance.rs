use std::ops::Add;

use cosmwasm_std::{Addr, Empty, StdResult, Uint128};
use cw721::NftInfoResponse;
use cw721_base::{ContractError::Ownership, OwnershipError::NotOwner};
use mars_account_nft::{
    error::{
        ContractError,
        ContractError::{BaseError, BurnNotAllowed, HealthContractNotSet},
    },
    msg::QueryMsg::NftInfo,
};

use crate::helpers::{below_max_for_burn, generate_health_response, MockEnv, MAX_VALUE_FOR_BURN};

pub mod helpers;

#[test]
fn only_token_owner_can_burn() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &below_max_for_burn());

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.burn(&bad_guy, &token_id);
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(err, BaseError(Ownership(NotOwner)));

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn burn_not_allowed_if_no_health_contract_set() {
    let mut mock = MockEnv::new().instantiate_with_health_contract(false).build().unwrap();
    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(error, HealthContractNotSet)
}

#[test]
fn burn_not_allowed_if_debt_balance() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(10_000, 0));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has a debt balance. Value: 10000.".to_string(),
        }
    )
}

#[test]
fn burn_not_allowed_if_too_much_collateral() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(
        &user,
        &token_id,
        &generate_health_response(0, MAX_VALUE_FOR_BURN.add(Uint128::one()).into()),
    );

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account collateral value exceeds config set max (1000). Total collateral value: 1001.".to_string()
        }
    )
}

#[test]
fn burn_allowance_at_exactly_max() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(
        &user,
        &token_id,
        &generate_health_response(0, MAX_VALUE_FOR_BURN.into()),
    );

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn burn_allowance_when_under_max() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 500));

    // Assert no errors on calling for NftInfo
    let _: NftInfoResponse<Empty> = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract.clone(),
            &NftInfo {
                token_id: token_id.clone(),
            },
        )
        .unwrap();

    mock.set_health_response(&user, &token_id, &below_max_for_burn());
    mock.burn(&user, &token_id).unwrap();

    let res: StdResult<NftInfoResponse<Empty>> = mock.app.wrap().query_wasm_smart(
        mock.nft_contract,
        &NftInfo {
            token_id,
        },
    );
    res.unwrap_err();
}
