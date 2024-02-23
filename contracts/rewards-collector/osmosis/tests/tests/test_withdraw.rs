use cosmwasm_std::{
    testing::mock_env, to_json_binary, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg,
};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::mock_info;
use mars_types::{
    credit_manager::{self, Action, ActionAmount, ActionCoin},
    rewards_collector::ExecuteMsg,
};

use super::helpers;

#[test]
fn withdrawing_from_red_bank() {
    let mut deps = helpers::setup_test();

    // anyone can execute a withdrawal
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::WithdrawFromRedBank {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "red_bank".to_string(),
            msg: to_json_binary(&mars_types::red_bank::ExecuteMsg::Withdraw {
                denom: "uatom".to_string(),
                amount: Some(Uint128::new(42069)),
                recipient: None,
                account_id: None,
                liquidation_related: None
            })
            .unwrap(),
            funds: vec![]
        }))
    )
}

#[test]
fn withdrawing_from_cm_if_action_not_allowed() {
    let mut deps = helpers::setup_test();

    // anyone can execute a withdrawal
    let error_res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::WithdrawFromCreditManager {
            account_id: "random_id".to_string(),
            actions: vec![
                Action::Withdraw(ActionCoin {
                    denom: "uatom".to_string(),
                    amount: ActionAmount::Exact(Uint128::new(100)),
                }),
                Action::WithdrawLiquidity {
                    lp_token: ActionCoin {
                        denom: "gamm/pool/1".to_string(),
                        amount: ActionAmount::AccountBalance,
                    },
                    slippage: Decimal::percent(5),
                },
                Action::RefundAllCoinBalances {},
            ],
        },
    )
    .unwrap_err();
    assert_eq!(error_res, ContractError::InvalidActionsForCreditManager {});
}

#[test]
fn withdrawing_from_cm_successfully() {
    let mut deps = helpers::setup_test();

    let account_id = "random_id".to_string();
    let actions = vec![
        Action::Withdraw(ActionCoin {
            denom: "uusdc".to_string(),
            amount: ActionAmount::Exact(Uint128::new(100)),
        }),
        Action::WithdrawLiquidity {
            lp_token: ActionCoin {
                denom: "gamm/pool/1".to_string(),
                amount: ActionAmount::AccountBalance,
            },
            slippage: Decimal::percent(5),
        },
        Action::Withdraw(ActionCoin {
            denom: "uatom".to_string(),
            amount: ActionAmount::Exact(Uint128::new(120)),
        }),
        Action::Withdraw(ActionCoin {
            denom: "uosmo".to_string(),
            amount: ActionAmount::Exact(Uint128::new(140)),
        }),
    ];

    // anyone can execute a withdrawal
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::WithdrawFromCreditManager {
            account_id: account_id.clone(),
            actions: actions.clone(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "credit_manager".to_string(),
            msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
                account_id,
                actions
            })
            .unwrap(),
            funds: vec![]
        }))
    )
}
