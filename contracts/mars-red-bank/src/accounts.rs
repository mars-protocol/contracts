use cosmwasm_std::{Addr, Decimal, Deps, StdResult, Uint128};

use mars_outpost::helpers::cw20_get_balance;
use mars_outpost::oracle;
use mars_outpost::red_bank::{Debt, User, UserHealthStatus};

use crate::error::ContractError;
use crate::helpers::{get_bit, market_get_from_index};
use crate::interest_rates::{get_underlying_debt_amount, get_underlying_liquidity_amount};
use crate::state::DEBTS;

/// User global position
pub struct UserPosition {
    pub total_collateral_in_base_asset: Uint128,
    pub total_debt_in_base_asset: Uint128,
    pub total_collateralized_debt_in_base_asset: Uint128,
    pub max_debt_in_base_asset: Uint128,
    pub weighted_liquidation_threshold_in_base_asset: Uint128,
    pub health_status: UserHealthStatus,
    pub asset_positions: Vec<UserAssetPosition>,
}

impl UserPosition {
    /// Gets asset price used to build the position for a given reference
    pub fn get_asset_price(&self, denom: &str) -> Result<Decimal, ContractError> {
        self.asset_positions
            .iter()
            .find(|ap| ap.denom == denom)
            .map(|ap| ap.asset_price)
            .ok_or_else(|| ContractError::PriceNotFound {
                denom: denom.to_string(),
            })
    }
}

/// User asset settlement
pub struct UserAssetPosition {
    pub denom: String,
    pub collateral_amount: Uint128,
    pub debt_amount: Uint128,
    pub uncollateralized_debt: bool,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub asset_price: Decimal,
}

/// Calculates the user data across the markets.
/// This includes the total debt/collateral balances in base asset,
/// the max debt in base asset, the average Liquidation threshold, and the Health factor.
pub fn get_user_position(
    deps: Deps,
    block_time: u64,
    user_address: &Addr,
    oracle_address: &Addr,
    user: &User,
    market_count: u32,
) -> StdResult<UserPosition> {
    let user_asset_positions = get_user_asset_positions(
        deps,
        market_count,
        user,
        user_address,
        oracle_address,
        block_time,
    )?;

    let mut total_collateral_in_base_asset = Uint128::zero();
    let mut total_debt_in_base_asset = Uint128::zero();
    let mut total_collateralized_debt_in_base_asset = Uint128::zero();
    let mut max_debt_in_base_asset = Uint128::zero();
    let mut weighted_liquidation_threshold_in_base_asset = Uint128::zero();

    for user_asset_position in &user_asset_positions {
        let asset_price = user_asset_position.asset_price;
        let collateral_in_base_asset = user_asset_position.collateral_amount * asset_price;
        total_collateral_in_base_asset =
            total_collateral_in_base_asset.checked_add(collateral_in_base_asset)?;

        max_debt_in_base_asset = max_debt_in_base_asset
            .checked_add(collateral_in_base_asset * user_asset_position.max_ltv)?;
        weighted_liquidation_threshold_in_base_asset = weighted_liquidation_threshold_in_base_asset
            .checked_add(collateral_in_base_asset * user_asset_position.liquidation_threshold)?;

        let debt_in_base_asset = user_asset_position.debt_amount * asset_price;
        total_debt_in_base_asset = total_debt_in_base_asset.checked_add(debt_in_base_asset)?;

        if !user_asset_position.uncollateralized_debt {
            total_collateralized_debt_in_base_asset =
                total_collateralized_debt_in_base_asset.checked_add(debt_in_base_asset)?;
        }
    }

    // When computing health factor we should not take debt into account that has been given
    // an uncollateralized loan limit
    let health_status = if total_collateralized_debt_in_base_asset.is_zero() {
        UserHealthStatus::NotBorrowing
    } else {
        let health_factor = Decimal::from_ratio(
            weighted_liquidation_threshold_in_base_asset,
            total_collateralized_debt_in_base_asset,
        );
        UserHealthStatus::Borrowing(health_factor)
    };

    let user_position = UserPosition {
        total_collateral_in_base_asset,
        total_debt_in_base_asset,
        total_collateralized_debt_in_base_asset,
        max_debt_in_base_asset,
        weighted_liquidation_threshold_in_base_asset,
        health_status,
        asset_positions: user_asset_positions,
    };

    Ok(user_position)
}

/// Goes through assets user has a position in and returns a vec containing the scaled debt
/// (denominated in the asset), a result from a specified computation for the current collateral
/// (denominated in asset) and some metadata to be used by the caller.
fn get_user_asset_positions(
    deps: Deps,
    market_count: u32,
    user: &User,
    user_address: &Addr,
    oracle_address: &Addr,
    block_time: u64,
) -> StdResult<Vec<UserAssetPosition>> {
    let mut ret: Vec<UserAssetPosition> = vec![];

    for i in 0_u32..market_count {
        let user_is_using_as_collateral = get_bit(user.collateral_assets, i)?;
        let user_is_borrowing = get_bit(user.borrowed_assets, i)?;
        if !(user_is_using_as_collateral || user_is_borrowing) {
            continue;
        }

        let (denom, market) = market_get_from_index(&deps, i)?;

        let (collateral_amount, max_ltv, liquidation_threshold) = if user_is_using_as_collateral {
            // query asset balance (ma_token contract gives back a scaled value)
            let asset_balance_scaled = cw20_get_balance(
                &deps.querier,
                market.ma_token_address.clone(),
                user_address.clone(),
            )?;

            let collateral_amount =
                get_underlying_liquidity_amount(asset_balance_scaled, &market, block_time)?;

            (collateral_amount, market.max_loan_to_value, market.liquidation_threshold)
        } else {
            (Uint128::zero(), Decimal::zero(), Decimal::zero())
        };

        let (debt_amount, uncollateralized_debt) = if user_is_borrowing {
            // query debt
            let user_debt: Debt = DEBTS.load(deps.storage, (&denom, user_address))?;

            let debt_amount =
                get_underlying_debt_amount(user_debt.amount_scaled, &market, block_time)?;

            (debt_amount, user_debt.uncollateralized)
        } else {
            (Uint128::zero(), false)
        };

        let asset_price = oracle::helpers::query_price(&deps.querier, oracle_address, &denom)?;

        let user_asset_position = UserAssetPosition {
            denom,
            collateral_amount,
            debt_amount,
            uncollateralized_debt,
            max_ltv,
            liquidation_threshold,
            asset_price,
        };
        ret.push(user_asset_position);
    }

    Ok(ret)
}
