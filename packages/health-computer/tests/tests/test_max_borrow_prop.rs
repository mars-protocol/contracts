use cosmwasm_std::{Addr, Decimal};
use mars_types::health::BorrowTarget;

use super::helpers::max_borrow_prop_test_runner;

#[test]
fn max_borrow_amount_deposit_renders_healthy_max_ltv() {
    max_borrow_prop_test_runner(2000, &BorrowTarget::Deposit);
}

#[test]
fn max_borrow_amount_wallet_renders_healthy_max_ltv() {
    max_borrow_prop_test_runner(2000, &BorrowTarget::Wallet);
}

#[test]
fn max_borrow_amount_vault_renders_healthy_max_ltv() {
    max_borrow_prop_test_runner(
        2000,
        &BorrowTarget::Vault {
            address: Addr::unchecked("123"),
        },
    );
}

#[test]
fn max_borrow_amount_swap_no_slippage_renders_healthy_max_ltv() {
    max_borrow_prop_test_runner(
        2000,
        &BorrowTarget::Swap {
            denom_out: "abc".to_string(),
            slippage: Decimal::zero(),
        },
    );
}

#[test]
fn max_borrow_amount_swap_renders_healthy_max_ltv() {
    max_borrow_prop_test_runner(
        2000,
        &BorrowTarget::Swap {
            denom_out: "abc".to_string(),
            slippage: Decimal::percent(1),
        },
    );
}
