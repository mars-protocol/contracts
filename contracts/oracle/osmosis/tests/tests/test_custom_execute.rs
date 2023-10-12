use cosmwasm_std::{testing::mock_env, Empty};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::contract::entry;
use mars_testing::mock_info;
use mars_types::oracle::ExecuteMsg;

use super::helpers;

#[test]
fn custom_execute() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::Custom(Empty {});
    let info = mock_info("owner");
    let res_err = entry::execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res_err, ContractError::MissingCustomExecuteParams {});
}
