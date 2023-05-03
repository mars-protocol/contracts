use cosmwasm_std::coin;
use mars_v3_zapper_base::{
    contract::{
        REFUND_AMOUNT_ATTR_KEY, REFUND_EVENT_TYPE, V3_POSITION_ATTR_KEY,
        V3_POSITION_CREATED_EVENT_TYPE,
    },
    msg::{ExecuteMsg, NewPositionRequest},
};
use osmosis_test_tube::{Account, Module, Wasm};

use crate::helpers::{assert_err, default_new_position_req, MockEnv};

pub mod helpers;

#[test]
fn only_owner_can_add_positions() {
    let mock = MockEnv::new().build().unwrap();
    let bad_guy = mock.app.init_account(&[coin(1_000_000, "uosmo")]).unwrap();

    let wasm = Wasm::new(&mock.app);
    let err = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(default_new_position_req()),
            &[],
            &bad_guy,
        )
        .unwrap_err();

    assert_err(err, "Caller is not owner");
}

#[test]
fn must_send_exact_funds() {
    let mut mock = MockEnv::new().build().unwrap();
    let (denom0, denom1, _) = mock.create_pool("ujuno", "umars");

    let wasm = Wasm::new(&mock.app);

    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: Some(coin(100_000_000, denom0.clone())),
        token_desired1: Some(coin(100_000_000, denom1.clone())),
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
    };

    let err = wasm
        .execute(&mock.zapper, &ExecuteMsg::CreatePosition(new_position.clone()), &[], &mock.owner)
        .unwrap_err();
    assert_err(err, "Sent fund mismatch");

    let err = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[new_position.token_desired0.clone().unwrap()],
            &mock.owner,
        )
        .unwrap_err();
    assert_err(err, "Sent fund mismatch");

    let err = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position),
            &[coin(1000, "uosmo")],
            &mock.owner,
        )
        .unwrap_err();
    assert_err(err, "Sent fund mismatch");

    // assert with None as well
    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: None,
        token_desired1: Some(coin(100_000_000, denom1)),
        token_min_amount0: "0".to_string(),
        token_min_amount1: "10000".to_string(),
    };

    let err = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[coin(100_000_000, denom0), new_position.token_desired1.unwrap()],
            &mock.owner,
        )
        .unwrap_err();
    assert_err(err, "Sent fund mismatch");
}

#[test]
fn add_position_successfully() {
    let mut mock = MockEnv::new().build().unwrap();
    let (denom0, denom1, _) = mock.create_pool("ujuno", "umars");

    let starting_balance = 100_000_000_000;

    // assert owner funds before
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    assert_eq!(starting_balance, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    assert_eq!(starting_balance, denom1_balance);

    // assert zapper funds before
    let denom0_balance = mock.query_balance(&mock.zapper, &denom0);
    assert_eq!(0, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.zapper, &denom1);
    assert_eq!(0, denom1_balance);

    let wasm = Wasm::new(&mock.app);

    let amount_sent = 100_000_000;
    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: Some(coin(amount_sent, denom0.clone())),
        token_desired1: Some(coin(amount_sent, denom1.clone())),
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
    };
    let res = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[
                new_position.token_desired0.clone().unwrap(),
                new_position.token_desired1.clone().unwrap(),
            ],
            &mock.owner,
        )
        .unwrap();

    // Assert correct event on logs
    let event = res
        .events
        .iter()
        .find(|e| e.ty == format!("wasm-{}", V3_POSITION_CREATED_EVENT_TYPE))
        .unwrap();
    let attr = event.attributes.iter().find(|a| a.key == V3_POSITION_ATTR_KEY).unwrap();
    assert_eq!("1", attr.value);

    // assert zapper has that position opened as expected
    let positions = mock.query_positions(1);
    assert_eq!(1, positions.len());
    let p = positions.first().unwrap();
    let position = p.position.clone().unwrap();
    assert_eq!(1, position.position_id);
    assert_eq!(mock.zapper, position.address);
    assert_eq!(1, position.pool_id);
    assert_eq!(new_position.lower_tick, position.lower_tick);
    assert_eq!(new_position.upper_tick, position.upper_tick);
    assert_eq!(new_position.token_desired0.unwrap().denom, p.asset0.clone().unwrap().denom);
    assert_eq!(new_position.token_desired1.unwrap().denom, p.asset1.clone().unwrap().denom);

    // assert zapper funds after
    let denom0_balance = mock.query_balance(&mock.zapper, &denom0);
    assert_eq!(0, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.zapper, &denom1);
    assert_eq!(0, denom1_balance);

    // assert owner funds after
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    let position_amount0 = p.asset0.clone().unwrap().amount.parse::<u128>().unwrap();
    assert_eq!(starting_balance - position_amount0, denom0_balance);

    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    let position_amount1 = p.asset1.clone().unwrap().amount.parse::<u128>().unwrap();
    assert_eq!(starting_balance - position_amount1, denom1_balance);
}

#[test]
fn refunds_are_issued() {
    let mut mock = MockEnv::new().build().unwrap();
    let (denom0, denom1, _) = mock.create_pool("ujuno", "umars");

    let starting_balance = 100_000_000_000;

    // assert owner funds before
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    assert_eq!(starting_balance, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    assert_eq!(starting_balance, denom1_balance);

    let wasm = Wasm::new(&mock.app);

    let amount_sent = 10_000_000;
    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -100,
        upper_tick: 100,
        token_desired0: Some(coin(amount_sent, denom0.clone())),
        token_desired1: Some(coin(amount_sent, denom1.clone())),
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
    };

    let res = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[new_position.token_desired0.unwrap(), new_position.token_desired1.unwrap()],
            &mock.owner,
        )
        .unwrap();

    // Zapper should not have a balance after tx
    let denom0_balance = mock.query_balance(&mock.zapper, &denom0);
    assert_eq!(0, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.zapper, &denom1);
    assert_eq!(0, denom1_balance);

    // Assert refund event emitted
    let refund_amount_a = 8992255u128;
    let event = res.events.iter().find(|e| e.ty == format!("wasm-{}", REFUND_EVENT_TYPE)).unwrap();
    let attr = event.attributes.iter().find(|a| a.key == REFUND_AMOUNT_ATTR_KEY).unwrap();
    assert_eq!(format!("{refund_amount_a}{denom1}"), attr.value);

    // No refund on denom0
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    assert_eq!(starting_balance - amount_sent, denom0_balance);

    // assert refund took place for denom1
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    assert_eq!(starting_balance - amount_sent + refund_amount_a, denom1_balance);

    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -100,
        upper_tick: -20,
        token_desired0: Some(coin(amount_sent, denom0.clone())),
        token_desired1: Some(coin(amount_sent, denom1.clone())),
        token_min_amount0: "0".to_string(),
        token_min_amount1: "10000".to_string(),
    };

    let res = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[new_position.token_desired0.unwrap(), new_position.token_desired1.unwrap()],
            &mock.owner,
        )
        .unwrap();

    // Assert refund event emitted
    let refund_amount_b = 10000000u128;
    let event = res.events.iter().find(|e| e.ty == format!("wasm-{}", REFUND_EVENT_TYPE)).unwrap();
    let attr = event.attributes.iter().find(|a| a.key == REFUND_AMOUNT_ATTR_KEY).unwrap();
    assert_eq!(format!("{refund_amount_b}{denom0}"), attr.value);

    // Full refund on denom0
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    // Starting balance after first position was created
    let balance_before = starting_balance - amount_sent;
    assert_eq!(balance_before, denom0_balance);

    // No refund for denom1
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    // Starting balance after first position was created
    let balance_before = starting_balance - amount_sent + refund_amount_a;
    assert_eq!(balance_before - amount_sent, denom1_balance);
}

#[test]
fn adding_multiple_increments() {
    let mut mock = MockEnv::new().build().unwrap();
    let (denom0, denom1, _) = mock.create_pool("ujuno", "umars");

    let wasm = Wasm::new(&mock.app);

    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: Some(coin(100_000_000, denom0)),
        token_desired1: Some(coin(100_000_000, denom1)),
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
    };
    wasm.execute(
        &mock.zapper,
        &ExecuteMsg::CreatePosition(new_position.clone()),
        &[
            new_position.token_desired0.clone().unwrap(),
            new_position.token_desired1.clone().unwrap(),
        ],
        &mock.owner,
    )
    .unwrap();

    wasm.execute(
        &mock.zapper,
        &ExecuteMsg::CreatePosition(new_position.clone()),
        &[
            new_position.token_desired0.clone().unwrap(),
            new_position.token_desired1.clone().unwrap(),
        ],
        &mock.owner,
    )
    .unwrap();

    let res = wasm
        .execute(
            &mock.zapper,
            &ExecuteMsg::CreatePosition(new_position.clone()),
            &[new_position.token_desired0.unwrap(), new_position.token_desired1.unwrap()],
            &mock.owner,
        )
        .unwrap();

    // Assert incrementing position id on logs
    let event = res
        .events
        .iter()
        .find(|e| e.ty == format!("wasm-{}", V3_POSITION_CREATED_EVENT_TYPE))
        .unwrap();
    let attr = event.attributes.iter().find(|a| a.key == V3_POSITION_ATTR_KEY).unwrap();
    assert_eq!("3", attr.value);
}

#[test]
fn error_rolls_back_state() {
    let mut mock = MockEnv::new().build().unwrap();
    let (denom0, denom1, _) = mock.create_pool("ujuno", "umars");

    let starting_balance = 100_000_000_000;

    // assert owner funds before
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    assert_eq!(starting_balance, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    assert_eq!(starting_balance, denom1_balance);

    let wasm = Wasm::new(&mock.app);

    let amount_sent = 100_000_000;
    let new_position = NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: Some(coin(amount_sent, denom0.clone())),
        token_desired1: Some(coin(amount_sent, denom1.clone())),
        token_min_amount0: "10000000000000000".to_string(),
        token_min_amount1: "10000".to_string(),
    };
    wasm.execute(
        &mock.zapper,
        &ExecuteMsg::CreatePosition(new_position.clone()),
        &[new_position.token_desired0.clone().unwrap(), new_position.token_desired1.unwrap()],
        &mock.owner,
    )
    .unwrap_err();

    // assert zapper funds after
    let denom0_balance = mock.query_balance(&mock.zapper, &denom0);
    assert_eq!(0, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.zapper, &denom1);
    assert_eq!(0, denom1_balance);

    // assert owner funds after
    let denom0_balance = mock.query_balance(&mock.owner.address(), &denom0);
    assert_eq!(starting_balance, denom0_balance);
    let denom1_balance = mock.query_balance(&mock.owner.address(), &denom1);
    assert_eq!(starting_balance, denom1_balance);
}
