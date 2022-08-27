use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coin, to_binary, CosmosMsg, Decimal, SubMsg, WasmMsg};
use mars_liquidation_filter::contract::execute;
use mars_liquidation_filter::ContractError;

use crate::helpers::setup_test;
use mars_outpost::liquidation_filter::msg::ExecuteMsg;
use mars_outpost::liquidation_filter::Liquidate;
use mars_outpost::red_bank::{UserHealthStatus, UserPositionResponse};

mod helpers;

// We are only interested in health status, the rest can have random values
fn dummy_user_position(health_status: UserHealthStatus) -> UserPositionResponse {
    UserPositionResponse {
        total_collateral_value: Default::default(),
        total_debt_value: Default::default(),
        total_collateralized_debt: Default::default(),
        weighted_max_ltv_collateral: Default::default(),
        weighted_liquidation_threshold_collateral: Default::default(),
        health_status,
    }
}

#[test]
fn test_liquidate_many_accounts_if_missing_debt_coin() {
    let mut deps = setup_test();
    deps.querier.set_redbank_user_position(
        "user_address_1".to_string(),
        dummy_user_position(UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::percent(80),
            liq_threshold_hf: Decimal::percent(90),
        }),
    );

    let info = mock_info("owner", &[coin(1234u128, "uosmo")]);
    let msg = ExecuteMsg::LiquidateMany {
        array: vec![
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "uosmo".to_string(),
                user_address: "user_address_1".to_string(),
                receive_ma_token: false,
            },
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "umars".to_string(),
                user_address: "user_address_2".to_string(),
                receive_ma_token: false,
            },
        ],
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::RequiredCoin {
            denom: "umars".to_string()
        }
    );
}

#[test]
fn test_liquidate_many_accounts() {
    let mut deps = setup_test();
    deps.querier.set_redbank_user_position(
        "user_address_1".to_string(),
        dummy_user_position(UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::percent(80),
            liq_threshold_hf: Decimal::percent(90),
        }),
    );
    deps.querier.set_redbank_user_position(
        "user_address_2".to_string(),
        dummy_user_position(UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::percent(110),
            liq_threshold_hf: Decimal::percent(120),
        }),
    );
    deps.querier.set_redbank_user_position(
        "user_address_3".to_string(),
        dummy_user_position(UserHealthStatus::Borrowing {
            max_ltv_hf: Decimal::percent(80),
            liq_threshold_hf: Decimal::percent(90),
        }),
    );
    deps.querier.set_redbank_user_position(
        "user_address_4".to_string(),
        dummy_user_position(UserHealthStatus::NotBorrowing),
    );

    let info = mock_info(
        "owner",
        &[
            coin(1234u128, "uosmo"),
            coin(2345u128, "umars"),
            coin(3456u128, "ujuno"),
            coin(7891u128, "uaxelar"),
        ],
    );
    let msg = ExecuteMsg::LiquidateMany {
        array: vec![
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "umars".to_string(),
                user_address: "user_address_1".to_string(),
                receive_ma_token: true,
            },
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "uosmo".to_string(),
                user_address: "user_address_2".to_string(),
                receive_ma_token: false,
            },
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "uaxelar".to_string(),
                user_address: "user_address_3".to_string(),
                receive_ma_token: false,
            },
            Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "ujuno".to_string(),
                user_address: "user_address_4".to_string(),
                receive_ma_token: false,
            },
        ],
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // user_address_2 is healthy, user_address_4 is not borrowing
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "red_bank".to_string(),
            msg: to_binary(&mars_outpost::red_bank::ExecuteMsg::Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "umars".to_string(),
                user_address: "user_address_1".to_string(),
                receive_ma_token: true
            })
            .unwrap(),
            funds: vec![coin(2345u128, "umars")]
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "red_bank".to_string(),
            msg: to_binary(&mars_outpost::red_bank::ExecuteMsg::Liquidate {
                collateral_denom: "uatom".to_string(),
                debt_denom: "uaxelar".to_string(),
                user_address: "user_address_3".to_string(),
                receive_ma_token: false
            })
            .unwrap(),
            funds: vec![coin(7891u128, "uaxelar")]
        }))
    );
}
