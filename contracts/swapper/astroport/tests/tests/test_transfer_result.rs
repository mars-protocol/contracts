use cosmwasm_std::{coin, Addr};
use cw_it::{
    test_tube::{Account, Module, Wasm},
    traits::CwItRunner,
};
use mars_swapper_astroport::{config::AstroportConfig, route::AstroportRoute};
use mars_testing::{astroport_swapper::AstroportSwapperRobot, test_runner::get_test_runner};

#[test]
fn transfer_result_unauthorized() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);
    let denom_in = "uosmo".to_string();
    let denom_out = "usd".to_string();

    let msg = mars_types::swapper::ExecuteMsg::<AstroportRoute, AstroportConfig>::TransferResult {
        recipient: Addr::unchecked(admin.address()),
        denom_in,
        denom_out,
    };

    let wasm = Wasm::new(&runner);
    assert!(wasm
        .execute(&robot.swapper, &msg, &[], &admin)
        .unwrap_err()
        .to_string()
        .contains("is not authorized"));
}
