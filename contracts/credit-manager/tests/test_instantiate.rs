use crate::helpers::{
    assert_contents_equal, uatom_info, ujake_info, unlocked_vault_info, uosmo_info, CoinInfo,
    MockEnv, VaultTestInfo,
};
use cosmwasm_std::{coin, Decimal};

pub mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let owner = "owner_addr";
    let mock = MockEnv::new().owner(owner).build().unwrap();
    let res = mock.query_config();
    assert_eq!(owner, res.owner);
}

#[test]
fn test_raises_on_invalid_owner_addr() {
    let owner = "%%%INVALID%%%";
    let res = MockEnv::new().owner(owner).build();
    if res.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_nft_contract_addr_not_set_on_instantiate() {
    let mock = MockEnv::new().no_nft_contract().build().unwrap();
    let res = mock.query_config();
    assert_eq!(res.account_nft, None);
}

#[test]
fn test_allowed_vaults_set_on_instantiate() {
    let allowed_vaults = vec![
        VaultTestInfo {
            vault_token_denom: "vault_contract_1".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
        },
        VaultTestInfo {
            vault_token_denom: "vault_contract_2".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
        },
        VaultTestInfo {
            vault_token_denom: "vault_contract_3".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
        },
    ];

    let mock = MockEnv::new()
        .allowed_vaults(&allowed_vaults)
        .build()
        .unwrap();
    let res = mock.query_vault_configs(None, None);
    assert_contents_equal(
        &res.iter().map(|v| v.vault.clone()).collect::<Vec<_>>(),
        &allowed_vaults
            .iter()
            .map(|info| mock.get_vault(info))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn test_raises_on_invalid_vaults_addr() {
    let mock = MockEnv::new()
        .pre_deployed_vault("%%%INVALID%%%", &unlocked_vault_info())
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_raises_on_invalid_vaults_config() {
    let mock = MockEnv::new()
        .pre_deployed_vault(
            "addr_123",
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
                base_token_denom: "lp_denom_123".to_string(),
            },
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error: max_ltv > liquidation_threshold");
    }

    let mock = MockEnv::new()
        .pre_deployed_vault(
            "addr_123",
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(9u128, 0).unwrap(),
                base_token_denom: "lp_denom_123".to_string(),
            },
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error: liquidation_threshold > 1");
    }
}

#[test]
fn test_allowed_coins_set_on_instantiate() {
    let allowed_coins = vec![
        uosmo_info(),
        uatom_info(),
        ujake_info(),
        CoinInfo {
            denom: "umars".to_string(),
            price: Decimal::from_atomics(25u128, 2).unwrap(),
            max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            liquidation_bonus: Decimal::from_atomics(2u128, 1).unwrap(),
        },
    ];
    let mock = MockEnv::new()
        .allowed_coins(&allowed_coins)
        .build()
        .unwrap();

    let res = mock.query_allowed_coins(None, None);
    assert_contents_equal(
        &res,
        &allowed_coins
            .iter()
            .map(|info| info.denom.clone())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn test_red_bank_set_on_instantiate() {
    let red_bank_addr = "mars_red_bank_contract_123".to_string();
    let mock = MockEnv::new().red_bank(&red_bank_addr).build().unwrap();
    let res = mock.query_config();
    assert_eq!(red_bank_addr, res.red_bank);
}

#[test]
fn test_raises_on_invalid_red_bank_addr() {
    let mock = MockEnv::new().red_bank("%%%INVALID%%%").build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_oracle_set_on_instantiate() {
    let oracle_adapter_contract = "oracle_contract_456".to_string();
    let mock = MockEnv::new()
        .oracle_adapter(&oracle_adapter_contract)
        .build()
        .unwrap();
    let res = mock.query_config();
    assert_eq!(oracle_adapter_contract, res.oracle);
}

#[test]
fn test_raises_on_invalid_oracle_addr() {
    let mock = MockEnv::new().oracle_adapter("%%%INVALID%%%").build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_max_close_factor_set_on_instantiate() {
    let mock = MockEnv::new().build().unwrap();
    let res = mock.query_config();
    let mock_default = Decimal::from_atomics(5u128, 1).unwrap();
    assert_eq!(mock_default, res.max_close_factor);
}
