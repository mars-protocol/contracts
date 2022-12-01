use cosmwasm_std::{Addr, Decimal, Empty, StdResult};
use cw721::NftInfoResponse;

use mars_account_nft::error::ContractError;
use mars_account_nft::error::ContractError::BurnNotAllowed;
use mars_account_nft::msg::QueryMsg::NftInfo;

use crate::helpers::{below_max_for_burn, generate_health_response, MockEnv, MAX_VALUE_FOR_BURN};

pub mod helpers;

#[test]
fn test_burn_not_allowed_if_too_many_debts() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(10_000, 0));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Decimal::from_atomics(10_000u128, 0).unwrap(),
            max_value_allowed: Decimal::from_atomics(MAX_VALUE_FOR_BURN, 0).unwrap()
        }
    )
}

#[test]
fn test_burn_not_allowed_if_too_much_collateral() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 10_000));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Decimal::from_atomics(10_000u128, 0).unwrap(),
            max_value_allowed: Decimal::from_atomics(MAX_VALUE_FOR_BURN, 0).unwrap()
        }
    )
}

#[test]
fn test_burn_allowance_works_with_both_debt_and_collateral() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(501, 500));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Decimal::from_atomics(1_001u128, 0).unwrap(),
            max_value_allowed: Decimal::from_atomics(MAX_VALUE_FOR_BURN, 0).unwrap()
        }
    )
}

#[test]
fn test_burn_allowance_at_exactly_max() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(500, 500));

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn test_burn_allowance_when_under_max() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(500, 500));

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

    let res: StdResult<NftInfoResponse<Empty>> = mock
        .app
        .wrap()
        .query_wasm_smart(mock.nft_contract, &NftInfo { token_id });
    res.unwrap_err();
}
