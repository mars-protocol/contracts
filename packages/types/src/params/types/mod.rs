use cosmwasm_std::Decimal;
use mars_utils::error::ValidationError;

pub mod asset;
pub mod hls;
pub mod vault;

/// liquidation_threshold should be greater than or equal to max_loan_to_value
pub fn assert_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (max LTV)", max_ltv),
        });
    }
    Ok(())
}

pub fn assert_hls_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "hls_liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (hls max LTV)", max_ltv),
        });
    }
    Ok(())
}
