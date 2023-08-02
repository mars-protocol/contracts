use cosmwasm_std::{Coin, StdResult, Uint128};
use mars_rover::{
    adapters::vault::{CoinValue, VaultPositionValue},
    msg::query::DebtAmount,
};
use mars_rover_health_computer::HealthComputer;
use mars_rover_health_types::BorrowTarget;
use proptest::{
    strategy::Strategy,
    test_runner::{Config, TestRunner},
};

use super::random_health_computer;

pub fn max_borrow_prop_test_runner(cases: u32, target: &BorrowTarget) {
    let config = Config::with_cases(cases);

    let mut runner = TestRunner::new(config);
    runner
        .run(
            &random_health_computer().prop_filter("At least one vault needs to be present", |h| {
                match target {
                    BorrowTarget::Vault {
                        ..
                    } => !h.positions.vaults.is_empty(),
                    _ => true,
                }
            }),
            |h| {
                let updated_target = match target {
                    BorrowTarget::Deposit => BorrowTarget::Deposit,
                    BorrowTarget::Wallet => BorrowTarget::Wallet,
                    BorrowTarget::Vault {
                        ..
                    } => {
                        let vault_position = h.positions.vaults.first().unwrap();
                        BorrowTarget::Vault {
                            address: vault_position.vault.address.clone(),
                        }
                    }
                };

                let denom_to_borrow = h.denoms_data.params.keys().next().unwrap();
                let max_borrow =
                    h.max_borrow_amount_estimate(denom_to_borrow, &updated_target).unwrap();

                let health_before = h.compute_health().unwrap();
                if health_before.is_above_max_ltv() {
                    assert_eq!(Uint128::zero(), max_borrow);
                } else {
                    let h_new = add_borrow(&h, denom_to_borrow, max_borrow, &updated_target)?;
                    let health_after = h_new.compute_health().unwrap();

                    // Ensure still healthy
                    assert!(!health_after.is_above_max_ltv(),);
                }
                Ok(())
            },
        )
        .unwrap();
}

fn add_borrow(
    h: &HealthComputer,
    denom: &str,
    amount: Uint128,
    target: &BorrowTarget,
) -> StdResult<HealthComputer> {
    let mut new_h = h.clone();

    new_h.positions.debts.push(DebtAmount {
        denom: denom.to_string(),
        shares: amount * Uint128::new(1000),
        amount,
    });

    match target {
        BorrowTarget::Deposit => {
            new_h.positions.deposits.push(Coin {
                denom: denom.to_string(),
                amount,
            });
        }
        BorrowTarget::Wallet => {}
        BorrowTarget::Vault {
            address,
        } => {
            let price = new_h.denoms_data.prices.get(denom).unwrap();
            let value = amount.mul_floor(*price);

            if let Some(vault_value) = new_h.vaults_data.vault_values.get_mut(address) {
                vault_value.vault_coin.value += value;
            } else {
                new_h.vaults_data.vault_values.insert(address.clone(), {
                    VaultPositionValue {
                        vault_coin: CoinValue {
                            denom: denom.to_string(),
                            amount,
                            value,
                        },
                        base_coin: CoinValue {
                            denom: denom.to_string(),
                            amount: Uint128::zero(),
                            value: Uint128::zero(),
                        },
                    }
                });
            }
        }
    }
    Ok(new_h)
}
