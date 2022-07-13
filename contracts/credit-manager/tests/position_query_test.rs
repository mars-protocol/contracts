use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_binary, Addr, OwnedDeps, Uint128};
use cw_asset::{AssetInfo, AssetInfoBase};

use credit_manager::contract::query;
use credit_manager::state::ASSETS;
use rover::msg::query::{PositionResponse, QueryMsg};

pub mod helpers;

#[test]
fn test_position_query_when_no_result() {
    let deps = mock_dependencies();
    let token_id = "token_id";
    let value = query_position(&deps, token_id);
    assert_eq!(value.token_id, token_id);
    assert_eq!(value.assets.len(), 0);
}

#[test]
fn test_position_query_when_assets_deposited() {
    let mut deps = mock_dependencies();

    let token_id = "token_id";
    let native_asset = AssetInfo::Native(String::from("native_asset"));
    let amount = Uint128::new(123);
    save_position(&mut deps, token_id, &native_asset, &amount);

    let value = query_position(&deps, token_id);
    assert_eq!(value.assets.len(), 1);
    assert_eq!(value.assets.first().unwrap().amount, amount);
    assert_eq!(value.assets.first().unwrap().info, native_asset.into());
}

#[test]
fn test_position_query_with_multiple_results() {
    let mut deps = mock_dependencies();

    let token_id_a = "token_id_a";
    let asset_a = AssetInfo::Native(String::from("asset_a"));
    let amount_a = Uint128::new(123);
    save_position(&mut deps, token_id_a, &asset_a, &amount_a);

    let asset_b = AssetInfo::Cw20(Addr::unchecked(String::from("asset_b")));
    let amount_b = Uint128::new(444);
    save_position(&mut deps, token_id_a, &asset_b, &amount_b);

    let asset_c = AssetInfo::Cw20(Addr::unchecked(String::from("asset_c")));
    let amount_c = Uint128::new(98);
    save_position(&mut deps, token_id_a, &asset_c, &amount_c);

    let token_id_b = "token_i_b";
    let amount_d = Uint128::new(567);
    save_position(&mut deps, token_id_b, &asset_a, &amount_d);

    let value = query_position(&deps, token_id_a);
    assert_eq!(value.assets.len(), 3);

    assert_present(&value, &asset_a, &amount_a);
    assert_present(&value, &asset_b, &amount_b);
    assert_present(&value, &asset_c, &amount_c);
}

fn assert_present(res: &PositionResponse, asset: &AssetInfoBase<Addr>, amount: &Uint128) {
    res.assets
        .iter()
        .find(|item| item.info == asset.clone().into() && &item.amount == amount)
        .unwrap();
}

fn save_position(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    token_id: &str,
    asset: &AssetInfoBase<Addr>,
    amount: &Uint128,
) {
    ASSETS
        .save(&mut deps.storage, (token_id, asset.into()), &amount)
        .unwrap();
}

fn query_position(
    deps: &OwnedDeps<MockStorage, MockApi, MockQuerier>,
    token_id: &str,
) -> PositionResponse {
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Position {
            token_id: token_id.into(),
        },
    )
    .unwrap();
    from_binary(&res).unwrap()
}
