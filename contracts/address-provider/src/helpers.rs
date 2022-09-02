use cosmwasm_std::{Addr, Api, StdResult};

use crate::error::ContractError;

/// Assert an address is valid
///
/// NOTE: The `deps.api.addr_validate` function can only verify addresses of the current chain, e.g.
/// a contract on Osmosis can only verify addresses with the `osmo1` prefix. If the provided address
/// does not start with this prefix, we simply skip the assertion. In such cases, the caller is responsible
/// in making sure the provided address is valid
pub(crate) fn assert_valid_addr(api: &dyn Api, human: &str, prefix: &str) -> StdResult<()> {
    if human.starts_with(prefix) {
        api.addr_validate(human)?;
    }
    Ok(())
}

/// Assert a message's sender is the contract's owner
pub(crate) fn assert_owner(sender: &Addr, owner: &str) -> Result<(), ContractError> {
    if *sender != owner {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}
