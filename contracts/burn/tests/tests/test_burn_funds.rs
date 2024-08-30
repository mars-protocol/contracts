use cosmwasm_std::{
    coins, from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Uint128,
};
use mars_burn_contract::contract::{execute, query};
use mars_types::burn::{BurntAmountResponse, ExecuteMsg, QueryMsg};

#[test]
fn burn_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("anyone", &coins(100, "token"));

    // Mint some tokens to the contract
    deps.querier.update_balance(env.contract.address.clone(), coins(100, "token"));

    let msg = ExecuteMsg::BurnFunds {
        denom: "token".to_string(),
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(1, res.messages.len());
    assert_eq!(
        res.messages[0].msg,
        BankMsg::Burn {
            amount: coins(100, "token")
        }
        .into()
    );

    // Check that the burnt amount was recorded
    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::GetBurntAmount {
            denom: "token".to_string(),
        },
    )
    .unwrap();
    let burnt_amount: BurntAmountResponse = from_json(res).unwrap();
    assert_eq!(burnt_amount.amount, Uint128::new(100));
}

#[test]
fn burn_no_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("anyone", &[]);

    let msg = ExecuteMsg::BurnFunds {
        denom: "token".to_string(),
    };
    let res = execute(deps.as_mut(), env, info, msg);

    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "Generic error: No funds to burn for denom: token");
}
