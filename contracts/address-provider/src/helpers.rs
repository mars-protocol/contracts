use cosmwasm_std::Api;

use crate::error::ContractError;

/// Assert an address is valid
///
/// NOTE: The `deps.api.addr_validate` function can only verify addresses of the current chain, e.g.
/// a contract on Osmosis can only verify addresses with the `osmo1` prefix. If the provided address
/// does not start with this prefix, we use bech32 decoding (valid address should be successfully decoded).
pub(crate) fn assert_valid_addr(
    api: &dyn Api,
    human: &str,
    prefix: &str,
) -> Result<(), ContractError> {
    if human.starts_with(prefix) {
        api.addr_validate(human)?;
    } else {
        bech32::decode(human).map_err(|_| ContractError::InvalidAddress(human.to_string()))?;
    }
    Ok(())
}

/// Prefix should be related to owner address prefix on a specific chain
pub(crate) fn assert_valid_prefix(owner: &str, prefix: &str) -> Result<(), ContractError> {
    if !owner.starts_with(prefix) {
        return Err(ContractError::InvalidChainPrefix(prefix.to_string()));
    }
    Ok(())
}
