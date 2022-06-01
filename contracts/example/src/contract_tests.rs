use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, from_binary};

use fields_credit_manager::example::{InstantiateMsg, QueryMsg, StoredStringResponse};

use crate::contract::{instantiate, query};

#[test]
fn test_proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &coins(1000, "luna"));

    let example_string = String::from("spiderman123");
    let res = instantiate(
        deps.as_mut(),
        mock_env(),
        info,
        InstantiateMsg {
            some_string: example_string.clone(),
        },
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetStoredString {}).unwrap();
    let value: StoredStringResponse = from_binary(&res).unwrap();
    assert_eq!(example_string, value.str);
}
