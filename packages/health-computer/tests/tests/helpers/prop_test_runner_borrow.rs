use cosmwasm_std::{Coin, Decimal, StdResult, Uint128};
use mars_rover_health_computer::HealthComputer;
use mars_types::{
    adapters::vault::{CoinValue, VaultPositionValue},
    credit_manager::DebtAmount,
    health::BorrowTarget,
};
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
                let mut keys = h.denoms_data.params.keys();
                let denom_to_borrow = keys.next().unwrap();
                let denom_to_swap_to = keys.next().unwrap();

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
                    BorrowTarget::Swap {
                        denom_out: _,
                        slippage,
                    } => BorrowTarget::Swap {
                        denom_out: denom_to_swap_to.clone(),
                        slippage: *slippage,
                    },
                };

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
        BorrowTarget::Swap {
            denom_out,
            slippage,
        } => {
            let price_in = new_h.denoms_data.prices.get(denom).unwrap();
            let price_out = new_h.denoms_data.prices.get(denom_out).unwrap();

            // denom_amount_out = (1 - slippage) * max_borrow_denom_amount * borrow_denom_price / denom_price_out
            // Use ceil math to avoid rounding errors in the test otheriwse we might end up with a health that is
            // slightly above max_ltv.
            let slippage = Decimal::one() - slippage;
            let amount = amount.mul_ceil(slippage);
            let value_in = amount.mul_ceil(*price_in);
            let amount_out = value_in.div_ceil(*price_out);

            new_h.positions.deposits.push(Coin {
                denom: denom_out.to_string(),
                amount: amount_out,
            });
        }
    }
    Ok(new_h)
}
