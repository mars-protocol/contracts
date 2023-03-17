use cosmwasm_std::{coin, Decimal};
use mars_rover::{
    adapters::vault::{VaultBase, VaultConfig},
    msg::instantiate::VaultInstantiateConfig,
};

use crate::helpers::{
    assert_contents_equal, locked_vault_info, uatom_info, ujake_info, unlocked_vault_info,
    uosmo_info, CoinInfo, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn owner_set_on_instantiate() {
    let owner = "owner_addr";
    let mock = MockEnv::new().owner(owner).build().unwrap();
    let res = mock.query_config();
    assert_eq!(owner, res.owner.unwrap());
}

#[test]
fn raises_on_invalid_owner_addr() {
    let owner = "%%%INVALID%%%";
    let res = MockEnv::new().owner(owner).build();
    if res.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn nft_contract_addr_not_set_on_instantiate() {
    let mock = MockEnv::new().no_nft_contract().build().unwrap();
    let res = mock.query_config();
    assert_eq!(res.account_nft, None);
}

#[test]
fn vault_configs_set_on_instantiate() {
    let vault_configs = vec![
        VaultTestInfo {
            vault_token_denom: "vault_contract_1".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
            whitelisted: true,
        },
        VaultTestInfo {
            vault_token_denom: "vault_contract_2".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
            whitelisted: true,
        },
        VaultTestInfo {
            vault_token_denom: "vault_contract_3".to_string(),
            lockup: None,
            base_token_denom: "lp_denom_123".to_string(),
            deposit_cap: coin(1_000_000, "uusdc"),
            max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
            whitelisted: true,
        },
    ];

    let mock = MockEnv::new().vault_configs(&vault_configs).build().unwrap();
    let res = mock.query_vault_configs(None, None);
    assert_contents_equal(
        &res.iter().map(|v| v.vault.clone()).collect::<Vec<_>>(),
        &vault_configs.iter().map(|info| mock.get_vault(info)).collect::<Vec<_>>(),
    );
}

#[test]
fn raises_on_invalid_vaults_addr() {
    let mock = MockEnv::new()
        .pre_deployed_vault(
            &unlocked_vault_info(),
            Some(VaultInstantiateConfig {
                vault: VaultBase {
                    address: "%%%INVALID%%%".to_string(),
                },
                config: VaultConfig {
                    deposit_cap: Default::default(),
                    max_ltv: Default::default(),
                    liquidation_threshold: Default::default(),
                    whitelisted: false,
                },
            }),
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn instantiate_raises_on_invalid_vaults_config() {
    let mock = MockEnv::new()
        .pre_deployed_vault(
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
                base_token_denom: "lp_denom_123".to_string(),
                whitelisted: true,
            },
            None,
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error: max_ltv > liquidation_threshold");
    }

    let mock = MockEnv::new()
        .pre_deployed_vault(
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(9u128, 0).unwrap(),
                base_token_denom: "lp_denom_123".to_string(),
                whitelisted: true,
            },
            None,
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error: liquidation_threshold > 1");
    }

    let mock = MockEnv::new()
        .pre_deployed_vault(
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(9u128, 0).unwrap(),
                base_token_denom: "lp_denom_123".to_string(),
                whitelisted: true,
            },
            None,
        )
        .pre_deployed_vault(
            &VaultTestInfo {
                vault_token_denom: "uleverage".to_string(),
                lockup: None,
                deposit_cap: coin(10_000_000, "uusdc"),
                max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
                base_token_denom: "xyz".to_string(),
                whitelisted: true,
            },
            None,
        )
        .build();

    if mock.is_ok() {
        panic!("Should have thrown an error: duplicate vault token denoms");
    }
}

#[test]
fn duplicate_vaults_raises() {
    let mock = MockEnv::new()
        .pre_deployed_vault(&locked_vault_info(), None)
        .pre_deployed_vault(&locked_vault_info(), None)
        .build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn allowed_coins_set_on_instantiate() {
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
    let mock = MockEnv::new().allowed_coins(&allowed_coins).build().unwrap();

    let res = mock.query_allowed_coins(None, None);
    assert_contents_equal(
        &res,
        &allowed_coins.iter().map(|info| info.denom.clone()).collect::<Vec<_>>(),
    )
}

#[test]
fn duplicate_coins_raises() {
    let allowed_coins = vec![uosmo_info(), uosmo_info(), uatom_info()];
    let mock = MockEnv::new().allowed_coins(&allowed_coins).build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn red_bank_set_on_instantiate() {
    let red_bank_addr = "mars_red_bank_contract_123".to_string();
    let mock = MockEnv::new().red_bank(&red_bank_addr).build().unwrap();
    let res = mock.query_config();
    assert_eq!(red_bank_addr, res.red_bank);
}

#[test]
fn raises_on_invalid_red_bank_addr() {
    let mock = MockEnv::new().red_bank("%%%INVALID%%%").build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn oracle_set_on_instantiate() {
    let oracle_contract = "oracle_contract_456".to_string();
    let mock = MockEnv::new().oracle(&oracle_contract).build().unwrap();
    let res = mock.query_config();
    assert_eq!(oracle_contract, res.oracle);
}

#[test]
fn raises_on_invalid_oracle_addr() {
    let mock = MockEnv::new().oracle("%%%INVALID%%%").build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn max_close_factor_set_on_instantiate() {
    let mock = MockEnv::new().build().unwrap();
    let res = mock.query_config();
    let mock_default = Decimal::from_atomics(5u128, 1).unwrap();
    assert_eq!(mock_default, res.max_close_factor);
}

#[test]
fn max_close_factor_validated() {
    let mock = MockEnv::new().max_close_factor(Decimal::from_atomics(1244u128, 3).unwrap()).build();

    if mock.is_ok() {
        panic!("Should have thrown an error: Max close factor should be below 1");
    }
}
