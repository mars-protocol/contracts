use mars_types::health::SwapKind;

use super::helpers::max_swap_prop_test_runner;

#[test]
fn max_swap_amount_default_renders_healthy_max_ltv() {
    max_swap_prop_test_runner(2000, &SwapKind::Default);
}

#[test]
fn max_swap_amount_margin_renders_healthy_max_ltv() {
    max_swap_prop_test_runner(2000, &SwapKind::Margin);
}
