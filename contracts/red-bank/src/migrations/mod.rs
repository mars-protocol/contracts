pub mod v1_1;

//------------------------------------------------------------------------------
// Below are helper functions for checking contract versions when migrating
// TODO: They should be upsteamed to cw2:
// https://github.com/CosmWasm/cw-plus/issues/857

use cosmwasm_std::{StdError, Storage};
use cw2::{ContractVersion, CONTRACT};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VersionError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("contract version info not found")]
    NotFound,

    #[error("wrong contract: expecting `{expected}`, found `{found}`")]
    WrongContract {
        expected: String,
        found: String,
    },

    #[error("wrong contract version: expecting `{expected}`, found `{found}`")]
    WrongVersion {
        expected: String,
        found: String,
    },
}

// TODO: upstream this to cw-plus
pub fn assert_version(
    storage: &dyn Storage,
    expected_contract: &str,
    expected_version: &str,
) -> Result<(), VersionError> {
    let Some(ContractVersion { contract, version }) = CONTRACT.may_load(storage)? else {
        return Err(VersionError::NotFound);
    };

    if contract != expected_contract {
        return Err(VersionError::WrongContract {
            expected: expected_contract.into(),
            found: contract,
        });
    }

    if version != expected_version {
        return Err(VersionError::WrongVersion {
            expected: expected_version.into(),
            found: version,
        });
    }

    Ok(())
}
