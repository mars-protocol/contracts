use crate::helpers::{default_asset_params, default_asset_params_with, is_user_liquidatable};
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_outpost::math;
use mars_outpost::red_bank::UserHealthStatus;
use mars_testing::integration::mock_env::MockEnvBuilder;

mod helpers;

#[test]
fn something() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uevmos", default_asset_params());

    // owner of contract executes SetAssetIncentive {} for uevmos
    // balance change action for user_xyz
    // simulate time passing so rewards accrue
    // claim rewards
    // Assert balances in Red bank
    // Assert balances in user wallet
}
