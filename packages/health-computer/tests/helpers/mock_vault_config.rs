use cosmwasm_std::{coin, Addr, Decimal};
use mars_params::types::vault::VaultConfig;

pub fn osmo_atom_1_config() -> VaultConfig {
    VaultConfig {
        addr: Addr::unchecked("osmoatom1"),
        deposit_cap: coin(1000000000000, "uatom"),
        max_loan_to_value: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(74u128, 2).unwrap(),
        whitelisted: true,
        hls: None,
    }
}
