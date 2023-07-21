use cosmwasm_std::Storage;
use cw2::{assert_contract_version, VersionError::WrongVersion};
use mars_rover::error::{ContractError::Version, ContractResult};

use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};

pub fn assert_migration_env(
    storage: &dyn Storage,
    old_version: &str,
    new_version: &str,
) -> ContractResult<()> {
    // Assert contract name & from-version is correct
    assert_contract_version(storage, CONTRACT_NAME, old_version)?;

    // Assert to-version is correct
    if CONTRACT_VERSION != new_version {
        return Err(Version(WrongVersion {
            expected: new_version.to_string(),
            found: CONTRACT_VERSION.to_string(),
        }));
    }

    Ok(())
}
