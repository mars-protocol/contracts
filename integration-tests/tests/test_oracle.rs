use crate::helpers::default_asset_params;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;

mod helpers;

#[test]
fn spot_test() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    //set price source to SPOT
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_spot(&mut mock_env, "uatom", 1);
    oracle.set_price_source_spot(&mut mock_env, "uosmo", 2);
    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());

    let atom_price = oracle.query_asset_price(&mut mock_env, "uatom");
    println!("{}", atom_price);
    // assert_ne!(atom_price, 1);

    let osmo_price = oracle.query_asset_price(&mut mock_env, "uosmo");
    println!("{}", osmo_price);
    // assert_ne!(osmo_price, 1);
}
