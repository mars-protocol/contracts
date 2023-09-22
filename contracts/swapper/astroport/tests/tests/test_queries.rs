use cosmwasm_std::coin;
use cw_it::{test_tube::Account, traits::CwItRunner};
use mars_testing::{astroport_swapper::AstroportSwapperRobot, test_runner::get_test_runner};

#[test]
fn query_owner() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let owner = robot.query_owner().unwrap();

    assert_eq!(owner, admin.address());
}
