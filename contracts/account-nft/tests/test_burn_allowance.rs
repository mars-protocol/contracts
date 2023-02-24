use cosmwasm_std::{Addr, Empty, StdResult, Uint128};
use cw721::NftInfoResponse;
use mars_account_nft::{
    error::{
        ContractError,
        ContractError::{BurnNotAllowed, HealthContractNotSet},
    },
    msg::QueryMsg::NftInfo,
};

use crate::helpers::{below_max_for_burn, generate_health_response, MockEnv, MAX_VALUE_FOR_BURN};

pub mod helpers;

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
fn burn_not_allowed_if_too_many_debts() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(10_000, 0));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Uint128::new(10_000),
            max_value_allowed: MAX_VALUE_FOR_BURN
        }
    )
}

#[test]
fn burn_not_allowed_if_too_much_collateral() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 10_000));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Uint128::new(10_000),
            max_value_allowed: MAX_VALUE_FOR_BURN
        }
    )
}

#[test]
fn burn_allowance_works_with_both_debt_and_collateral() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(501, 500));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            current_balances: Uint128::new(1_001),
            max_value_allowed: MAX_VALUE_FOR_BURN
        }
    )
}

#[test]
fn burn_allowance_at_exactly_max() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(500, 500));

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn burn_allowance_when_under_max() {
    let mut mock = MockEnv::new().build().unwrap();

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

    let res: StdResult<NftInfoResponse<Empty>> = mock.app.wrap().query_wasm_smart(
        mock.nft_contract,
        &NftInfo {
            token_id,
        },
    );
    res.unwrap_err();
}
