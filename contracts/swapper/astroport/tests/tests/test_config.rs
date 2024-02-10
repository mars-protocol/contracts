use cosmwasm_std::coin;
use cw_it::traits::CwItRunner;
use mars_swapper_astroport::config::AstroportConfig;
use mars_testing::{astroport_swapper::AstroportSwapperRobot, test_runner::get_test_runner};

#[test]
#[should_panic]
fn set_config_not_admin() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let caller = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    robot.set_config(
        AstroportConfig {
            router: "router_123_contract".to_string(),
            factory: "factory_456_contract".to_string(),
            oracle: "oracle_789_contract".to_string(),
        },
        &caller,
    );
}

#[test]
fn query_config() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let astro_config = AstroportConfig {
        router: "router_123_contract".to_string(),
        factory: "factory_456_contract".to_string(),
        oracle: "oracle_789_contract".to_string(),
    };
    robot.set_config(astro_config.clone(), &admin);

    let config = robot.query_config();

    assert_eq!(config, astro_config);
}
