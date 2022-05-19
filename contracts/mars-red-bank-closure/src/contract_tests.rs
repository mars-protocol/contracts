use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, CosmosMsg, OwnedDeps, ReplyOn, SubMsg, Uint128, WasmMsg,
};

use mars_core::asset::Asset;
use mars_core::ma_token::msg::ExecuteMsg as MaTokenExecuteMsg;
use mars_core::testing::{mock_dependencies, MarsMockQuerier};
use mars_red_bank::state::MARKETS;
use mars_red_bank::Market;

use crate::contract::execute;
use crate::msg::ExecuteMsg;

fn uusd() -> Asset {
    Asset::Native {
        denom: "uusd".to_string(),
    }
}

// contract has 120 UST
//
// maUST:
// total supply: 100
// alice: 60
// bob: 30
// charlie: 10
fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(&[coin(120000000, "uusd")]);

    deps.querier.set_cw20_balances(
        Addr::unchecked("maUST"),
        &[
            (Addr::unchecked("alice"), Uint128::new(60000000)),
            (Addr::unchecked("bob"), Uint128::new(30000000)),
            (Addr::unchecked("charlie"), Uint128::new(10000000)),
        ],
    );
    deps.querier
        .set_cw20_total_supply(Addr::unchecked("maUST"), Uint128::new(100000000));

    MARKETS
        .save(
            deps.as_mut().storage,
            &uusd().get_reference(),
            &Market {
                ma_token_address: Addr::unchecked("maUST"),
                debt_total_scaled: Uint128::zero(),
                ..Default::default()
            },
        )
        .unwrap();

    deps
}

#[test]
fn refunding() {
    let mut deps = setup_test();

    // alice gets 120000000 * 60000000 / 100000000 = 72000000
    // total_amount_to_refund = 120000000 - 72000000 = 48000000
    // maUST total supply = 10000000 - 60000000 = 40000000
    //
    // bob gets 48000000 * 30000000 / 40000000 = 36000000
    // total_amount_to_refund = 48000000 - 36000000 = 12000000
    // maUST total supply = = 40000000 - 30000000 = 10000000
    //
    // charlie gets the rest 12000000 uusd
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("admin", &[]),
        ExecuteMsg::Refund { asset: uusd() },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 6);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "maUST".to_string(),
                msg: to_binary(&MaTokenExecuteMsg::Burn {
                    user: "alice".to_string(),
                    amount: Uint128::new(60000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "alice".to_string(),
                amount: vec![coin(72000000, "uusd")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[2],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "maUST".to_string(),
                msg: to_binary(&MaTokenExecuteMsg::Burn {
                    user: "bob".to_string(),
                    amount: Uint128::new(30000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "bob".to_string(),
                amount: vec![coin(36000000, "uusd")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[4],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "maUST".to_string(),
                msg: to_binary(&MaTokenExecuteMsg::Burn {
                    user: "charlie".to_string(),
                    amount: Uint128::new(10000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[5],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "charlie".to_string(),
                amount: vec![coin(12000000, "uusd")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
}
