use cosmwasm_std::{Addr, DepsMut, Empty, MessageInfo, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{
    contract::{Parent, CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
};

const FROM_VERSION: &str = "2.0.0";

const CREDIT_MANAGER_CONTRACT_ADDR: &str =
    "osmo1f2m24wktq0sw3c0lexlg7fv4kngwyttvzws3a3r3al9ld2s2pvds87jqvf";

/// Check Credit-Manager `config` query response:
/// {
///     ...
///       "rewards_collector": {
///         "address": "osmo1urvqe5mw00ws25yqdd4c4hlh8kdyf567mpcml7cdve9w08z0ydcqvsrgdy",
///         "account_id": "2321"
///       }
///     }
/// }
const REWARDS_COLLECTOR_CONTRACT_ADDR: &str =
    "osmo1urvqe5mw00ws25yqdd4c4hlh8kdyf567mpcml7cdve9w08z0ydcqvsrgdy";
const REWARDS_COLLECTOR_ACC_ID: &str = "2321";

pub fn migrate(mut deps: DepsMut) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Create missing connection between Reward-Collector contract addr and acc id
    Parent::default().mint(
        deps.branch(),
        MessageInfo {
            sender: Addr::unchecked(CREDIT_MANAGER_CONTRACT_ADDR.to_string()),
            funds: vec![],
        },
        REWARDS_COLLECTOR_ACC_ID.to_string(),
        REWARDS_COLLECTOR_CONTRACT_ADDR.to_string(),
        None,
        Empty {},
    )?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
