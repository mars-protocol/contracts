use cosmwasm_std::{coin, Coin};
use osmosis_std::types::osmosis::{
    concentratedliquidity::v1beta1::MsgCreateConcentratedPool, tokenfactory::v1beta1::MsgMint,
};
use osmosis_test_tube::{
    osmosis_std::types::osmosis::tokenfactory::v1beta1::MsgCreateDenom, Account,
    ConcentratedLiquidity, Module, OsmosisTestApp, TokenFactory,
};

pub mod helpers;

#[test]
fn add_position() {
    let app = OsmosisTestApp::new();

    let cl = ConcentratedLiquidity::new(&app);
    let token_factory = TokenFactory::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uatom"), coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let signer = &accs[0];

    let denom0 = token_factory
        .create_denom(
            MsgCreateDenom {
                sender: signer.address(),
                subdenom: "xyz".to_string(),
            },
            signer,
        )
        .unwrap()
        .data
        .new_token_denom;

    let denom1 = token_factory
        .create_denom(
            MsgCreateDenom {
                sender: signer.address(),
                subdenom: "abc".to_string(),
            },
            signer,
        )
        .unwrap()
        .data
        .new_token_denom;

    token_factory
        .mint(
            MsgMint {
                sender: signer.address(),
                amount: Some(Coin::new(100_000_000_000, &denom0).into()),
                mint_to_address: signer.address(),
            },
            signer,
        )
        .unwrap();

    token_factory
        .mint(
            MsgMint {
                sender: signer.address(),
                amount: Some(Coin::new(100_000_000_000, &denom1).into()),
                mint_to_address: signer.address(),
            },
            signer,
        )
        .unwrap();

    let pool_id = cl
        .create_concentrated_pool(
            MsgCreateConcentratedPool {
                sender: signer.address(),
                denom0,
                denom1,
                tick_spacing: 1,
                exponent_at_price_one: "-4".to_string(),
                swap_fee: "0".to_string(),
            },
            signer,
        )
        .unwrap()
        .data
        .pool_id;

    assert_eq!(1, pool_id)

    // TODO: Temporarily broken. Waiting for latest.
    // let position_id = cl
    //     .create_position(
    //         MsgCreatePosition {
    //             pool_id,
    //             sender: signer.address(),
    //             lower_tick: -1,
    //             upper_tick: 100,
    //             token_desired0: Some(v1beta1::Coin {
    //                 denom: denom0.to_string(),
    //                 amount: "9999999999".to_string(),
    //             }),
    //             token_desired1: Some(v1beta1::Coin {
    //                 denom: denom1.to_string(),
    //                 amount: "10000000000".to_string(),
    //             }),
    //             token_min_amount0: "1".to_string(),
    //             token_min_amount1: "1".to_string(),
    //             freeze_duration: None,
    //         },
    //         signer,
    //     )
    //     .unwrap()
    //     .data
    //     .position_id;
    //
    // println!("Position id: {position_id}");

    // let liquidity = cl
    //     .query_total_liquidity_for_range(&QueryTotalLiquidityForRangeRequest {
    //         pool_id,
    //     })
    //     .unwrap();

    // for l in liquidity {
    //     println!("liquidity_amount: {}", l.liquidity_amount);
    // }
}
