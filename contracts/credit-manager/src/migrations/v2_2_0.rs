use cosmwasm_std::{Decimal, DepsMut, Response};
use cw2::{assert_contract_version, get_contract_version, set_contract_version, VersionError};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::SWAP_FEE,
};

const FROM_VERSION: &str = "2.1.0";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    let contract = format!("crates.io:{CONTRACT_NAME}");
    let version = get_contract_version(deps.storage)?;
    let from_version = version.version;

    if version.contract != contract {
        return Err(ContractError::Version(VersionError::WrongContract {
            expected: contract,
            found: version.contract,
        }));
    }

    if from_version != FROM_VERSION {
        return Err(ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: from_version,
        }));
    }

    assert_contract_version(deps.storage, &contract, FROM_VERSION)?;

    if SWAP_FEE.may_load(deps.storage)?.is_none() {
        SWAP_FEE.save(deps.storage, &Decimal::zero())?;
    }

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", from_version)
        .add_attribute("to_version", CONTRACT_VERSION))
}
