use std::{collections::HashMap, ops::Add, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_rover::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultConfig,
        VaultPosition, VaultPositionAmount, VaultPositionValue, VaultUnlockingPosition,
    },
    msg::query::{DebtAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};

use crate::helpers::{udai_info, ujuno_info, uluna_info, umars_info, ustars_info};

pub mod helpers;

/// Action: User deposits 300 mars (1 price)
/// Health: assets_value: 300
///         debt value 0
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn only_assets_with_no_debts() {
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.market.denom.clone(), umars.price)]),
        markets: HashMap::from([(umars.market.denom.clone(), umars.market.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: umars.market.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom.clone()],
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(umars.price).unwrap();
    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.market.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.market.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Step 1: User deposits 12 luna (100 price) and borrows 2 luna
/// Health: assets_value: 1400
///         debt value 200
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: luna price goes to zero
/// Health: assets_value: 0
///         debt value 0 (still debt shares outstanding)
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn terra_ragnarok() {
    let mut uluna = uluna_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(uluna.market.denom.clone(), uluna.price)]),
        markets: HashMap::from([(uluna.market.denom.clone(), uluna.market.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(12);
    let borrow_amount = Uint128::new(3);

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.market.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.market.denom.clone(),
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data: vaults_data.clone(),
        allowed_coins: vec![uluna.market.denom.clone()],
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(uluna.price).unwrap();
    let debts_value = borrow_amount.checked_mul_floor(uluna.price).unwrap();

    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.market.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.market.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, borrow_amount.checked_mul_floor(uluna.price).unwrap());
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.market.liquidation_threshold).unwrap(),
            debts_value
        ))
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.market.max_loan_to_value).unwrap(),
            debts_value,
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Terra implosion
    uluna.price = Decimal::zero();

    let denoms_data = DenomsData {
        prices: HashMap::from([(uluna.market.denom.clone(), uluna.price)]),
        markets: HashMap::from([(uluna.market.denom.clone(), uluna.market.clone())]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.market.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.market.denom.clone(),
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![uluna.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Actions: User deposits 300 stars
///          and borrows 49 juno
/// Health: assets_value: 1569456334491.12991516325
///         debt value 350615100.25
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn ltv_and_lqdt_adjusted_values() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.market.denom.clone(), ustars.price),
            (ujuno.market.denom.clone(), ujuno.price),
        ]),
        markets: HashMap::from([
            (ustars.market.denom.clone(), ustars.market.clone()),
            (ujuno.market.denom.clone(), ujuno.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let borrow_amount = Uint128::new(49);

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.market.denom.clone(),
                    amount: deposit_amount,
                },
                Coin {
                    denom: ujuno.market.denom.clone(),
                    amount: borrow_amount,
                },
            ],
            debts: vec![DebtAmount {
                denom: ujuno.market.denom.clone(),
                shares: Uint128::new(12345),
                amount: borrow_amount.add(Uint128::one()), // simulated interest
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![ustars.market.denom.clone(), ujuno.market.denom.clone()],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(
        health.total_collateral_value,
        deposit_amount
            .checked_mul_floor(ustars.price)
            .unwrap()
            .add(borrow_amount.checked_mul_floor(ujuno.price).unwrap())
    );
    assert_eq!(
        health.total_debt_value,
        Uint128::new(350_615_100) // with simulated interest
    );
    let lqdt_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.market.liquidation_threshold)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.market.liquidation_threshold)
                .unwrap(),
        );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            lqdt_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_floor(ujuno.price).unwrap()
        ))
    );
    let ltv_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.market.max_loan_to_value)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.market.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            ltv_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_floor(ujuno.price).unwrap()
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Borrows 30 stars
/// Borrows 49 juno
/// Deposits 298 stars
/// Test validates debt calculation results
#[test]
fn debt_value() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.market.denom.clone(), ustars.price),
            (ujuno.market.denom.clone(), ujuno.price),
        ]),
        markets: HashMap::from([
            (ustars.market.denom.clone(), ustars.market.clone()),
            (ujuno.market.denom.clone(), ujuno.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount_stars = Uint128::new(298);
    let borrowed_amount_juno = Uint128::new(49);
    let borrowed_amount_stars = Uint128::new(30);

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.market.denom.clone(),
                    amount: deposit_amount_stars,
                },
                Coin {
                    denom: ujuno.market.denom.clone(),
                    amount: borrowed_amount_juno,
                },
                Coin {
                    denom: ustars.market.denom.clone(),
                    amount: borrowed_amount_stars,
                },
            ],
            debts: vec![
                DebtAmount {
                    denom: ujuno.market.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_juno.add(Uint128::one()), // simulated interest
                },
                DebtAmount {
                    denom: ustars.market.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_stars.add(Uint128::one()), // simulated interest
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![ustars.market.denom.clone(), ujuno.market.denom.clone()],
    };

    let health = h.compute_health().unwrap();

    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());

    let juno_debt_value =
        borrowed_amount_juno.add(Uint128::one()).checked_mul_floor(ujuno.price).unwrap();

    let stars_debt_value =
        borrowed_amount_stars.add(Uint128::one()).checked_mul_floor(ustars.price).unwrap();

    let total_debt_value = juno_debt_value.add(stars_debt_value);
    assert_eq!(health.total_debt_value, total_debt_value);

    let lqdt_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.market.liquidation_threshold)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.market.liquidation_threshold)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.market.liquidation_threshold)
                .unwrap(),
        );

    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(lqdt_adjusted_assets_value, total_debt_value))
    );

    let max_ltv_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.market.max_loan_to_value)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.market.max_loan_to_value)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.market.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(max_ltv_adjusted_assets_value, total_debt_value))
    );
}

#[test]
fn above_max_ltv_below_liq_threshold() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![DebtAmount {
                denom: udai.market.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(971));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.996910401647785787").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.047373841400617919").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn liquidatable() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.826643894107600341").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.868488471391972672").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn allowed_coins_influence_max_ltv() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(960));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.819812126387702818").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.868488471391972672").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn unlocked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.market.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.624252775405636208").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.116140051238257899").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.market.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(42451613)),
                    unlocking: UnlockingPositions::new(vec![]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.624252775405636208").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.116140051238257899").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault_with_unlocking_positions() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: udai.market.denom.clone(),
                    amount: Default::default(),
                    value: Uint128::new(264),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(840, udai.market.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(3, udai.market.denom.clone()),
                        },
                    ]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3192));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3754));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.72587532023911187").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.205807002561912894").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn vault_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.market.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: false,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.826643894107600341").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.116140051238257899").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

/// Delisting base token will make even vault token maxLTV to drop
#[test]
fn vault_base_token_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();
    let ujuno = ujuno_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
            (ujuno.market.denom.clone(), ujuno.price),
        ]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
            (ujuno.market.denom.clone(), ujuno.market.clone()),
        ]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: ujuno.market.denom.clone(),
                    amount: Default::default(),
                    value: Uint128::new(497873442),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(60, ujuno.market.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(11, ujuno.market.denom),
                        },
                    ]),
                }),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom],
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(497879652));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968)); // Lower due to vault blacklisted
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(448089614));
    assert_eq!(health.total_debt_value, Uint128::new(1171));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.826643894107600341").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("382655.520068317677198975").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}
