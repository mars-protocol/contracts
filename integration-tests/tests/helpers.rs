use cosmwasm_std::Decimal;
use mars_outpost::red_bank::{InitOrUpdateAssetParams, InterestRateModel};

pub fn default_asset_params() -> InitOrUpdateAssetParams {
    InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::percent(10)),
        reserve_factor: Some(Decimal::percent(20)),
        max_loan_to_value: Some(Decimal::percent(60)),
        liquidation_threshold: Some(Decimal::percent(80)),
        liquidation_bonus: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    }
}

pub fn default_asset_params_with(
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
) -> InitOrUpdateAssetParams {
    InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::percent(10)),
        reserve_factor: Some(Decimal::percent(20)),
        max_loan_to_value: Some(max_loan_to_value),
        liquidation_threshold: Some(liquidation_threshold),
        liquidation_bonus: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        }),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    }
}
