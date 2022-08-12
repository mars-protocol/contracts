use std::collections::{HashMap, HashSet};

use cosmwasm_std::{Addr, Decimal, Deps, Order, StdResult, Uint128};

use mars_outpost::oracle;
use mars_outpost::red_bank::UserHealthStatus;

use crate::error::ContractError;
use crate::interest_rates::{get_underlying_debt_amount, get_underlying_liquidity_amount};
use crate::state::{COLLATERALS, DEBTS, MARKETS};

/// User global position
pub struct UserPosition {
    pub total_collateral_in_base_asset: Uint128,
    pub total_debt_in_base_asset: Uint128,
    pub total_collateralized_debt_in_base_asset: Uint128,
    pub max_debt_in_base_asset: Uint128,
    pub weighted_liquidation_threshold_in_base_asset: Uint128,
    pub health_status: UserHealthStatus,
    pub asset_positions: HashMap<String, UserAssetPosition>,
}

impl UserPosition {
    /// Gets asset price used to build the position for a given reference
    pub fn get_asset_price(&self, denom: &str) -> Result<Decimal, ContractError> {
        self.asset_positions.get(denom).map(|ap| ap.asset_price).ok_or_else(|| {
            ContractError::PriceNotFound {
                denom: denom.to_string(),
            }
        })
    }
}

#[derive(Default)]
pub struct UserAssetPosition {
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
) -> StdResult<UserPosition> {
    let user_asset_positions =
        get_user_asset_positions(deps, user_address, oracle_address, block_time)?;

    let mut total_collateral_in_base_asset = Uint128::zero();
    let mut total_debt_in_base_asset = Uint128::zero();
    let mut total_collateralized_debt_in_base_asset = Uint128::zero();
    let mut max_debt_in_base_asset = Uint128::zero();
    let mut weighted_liquidation_threshold_in_base_asset = Uint128::zero();

    for user_asset_position in user_asset_positions.values() {
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

/// Goes through assets user has a position in and returns a HashMap mapping the asset denoms to the
/// scaled amounts, and some metadata to be used by the caller.
fn get_user_asset_positions(
    deps: Deps,
    user_address: &Addr,
    oracle_address: &Addr,
    block_time: u64,
) -> StdResult<HashMap<String, UserAssetPosition>> {
    // Firstly, find all denoms that the user has a collateral or debt position in.
    // Collect them into a hashset so there are no dups.
    let collateral_denoms = COLLATERALS
        .prefix(user_address)
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<HashSet<_>>>()?;
    let debt_denoms = DEBTS
        .prefix(user_address)
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<HashSet<_>>>()?;

    let mut denoms = HashSet::new();
    denoms.extend(collateral_denoms);
    denoms.extend(debt_denoms);

    // Then, enumerate all denoms, compute the underlying amounts, and query the prices.
    //
    // Finally, collect the results into a hashmap indexed by the denoms.
    //
    // NOTE: a collateral asset is only included if its collateral status is set to "active".
    denoms
        .into_iter()
        .map(|denom| {
            let mut p = UserAssetPosition::default();

            let market = MARKETS.load(deps.storage, &denom)?;
            p.max_ltv = market.max_loan_to_value;
            p.liquidation_threshold = market.liquidation_threshold;

            if let Some(collateral) = COLLATERALS.may_load(deps.storage, (user_address, &denom))? {
                if collateral.enabled && !collateral.amount_scaled.is_zero() {
                    p.collateral_amount = get_underlying_liquidity_amount(
                        collateral.amount_scaled,
                        &market,
                        block_time,
                    )?;
                }
            }

            if let Some(debt) = DEBTS.may_load(deps.storage, (user_address, &denom))? {
                if !debt.amount_scaled.is_zero() {
                    p.debt_amount =
                        get_underlying_debt_amount(debt.amount_scaled, &market, block_time)?;
                    p.uncollateralized_debt = debt.uncollateralized;
                }
            }

            p.asset_price = oracle::helpers::query_price(&deps.querier, oracle_address, &denom)?;

            Ok((denom, p))
        })
        .collect()
}
