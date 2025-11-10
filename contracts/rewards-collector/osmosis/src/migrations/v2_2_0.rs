use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, get_contract_version, set_contract_version, VersionError};
use mars_rewards_collector_base::ContractError;

use crate::entry::{CONTRACT_NAME, CONTRACT_VERSION};

const FROM_VERSION: &str = "2.1.1";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    let contract = format!("crates.io:{CONTRACT_NAME}");
    let version = get_contract_version(deps.storage)?;

    if version.contract != contract {
        return Err(ContractError::Version(VersionError::WrongContract {
            expected: contract,
            found: version.contract,
        }));
    }

    if version.version != FROM_VERSION {
        return Err(ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: version.version,
        }));
    }

    assert_contract_version(deps.storage, &contract, FROM_VERSION)?;

    set_contract_version(deps.storage, contract, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
