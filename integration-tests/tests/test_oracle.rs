use crate::helpers::default_asset_params;
use cosmwasm_std::{Addr, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;
use osmosis_std::types::osmosis::gamm::v1beta1::QuerySpotPriceResponse;
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;

mod helpers;

#[test]
fn spot_test() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    //setup oracle with SPOT price source
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_spot(&mut mock_env, "uatom", 1);

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());

    let atom_price = oracle.query_asset_price(&mut mock_env, "uatom");
    assert_ne!(atom_price, Uint128::zero());
}

fn twap_test() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    //setup oracle with TWAP price source
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_twap(&mut mock_env, "uatom", 1, 1800);
}
