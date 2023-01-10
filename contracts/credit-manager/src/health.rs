use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Deps, Env, Event, Response, Uint128};
use mars_health::Health;

use mars_math::FractionMath;
use mars_outpost::oracle::PriceResponse;
use mars_outpost::red_bank::Market;
use mars_rover::adapters::vault::VaultPosition;
use mars_rover::error::{ContractError, ContractResult};
use mars_rover::msg::query::{DebtAmount, Positions};

use crate::query::query_positions;
use crate::state::{ALLOWED_COINS, ORACLE, RED_BANK, VAULT_CONFIGS};

/// Used as storage when trying to compute Health
#[cw_serde]
struct CollateralValue {
    pub total_collateral_value: Uint128,
    pub max_ltv_adjusted_collateral: Uint128,
    pub liquidation_threshold_adjusted_collateral: Uint128,
}

/// The mars-health package, red bank, and oracle do not have knowledge of vault config or pricing.
/// Cannot use the health package so forking and adjusting for rover internally here.
pub fn compute_health(deps: Deps, env: &Env, account_id: &str) -> ContractResult<Health> {
    let positions = query_positions(deps, env, account_id)?;

    let CollateralValue {
        total_collateral_value,
        max_ltv_adjusted_collateral,
        liquidation_threshold_adjusted_collateral,
    } = calculate_collateral_value(&deps, &positions)?;

    let total_debt_value = calculate_total_debt_value(&deps, &positions.debts)?;

    let max_ltv_health_factor = if total_debt_value.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(
            max_ltv_adjusted_collateral,
            total_debt_value,
        )?)
    };

    let liquidation_health_factor = if total_debt_value.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(
            liquidation_threshold_adjusted_collateral,
            total_debt_value,
        )?)
    };

    Ok(Health {
        total_debt_value,
        total_collateral_value,
        max_ltv_adjusted_collateral,
        liquidation_threshold_adjusted_collateral,
        max_ltv_health_factor,
        liquidation_health_factor,
    })
}

fn calculate_collateral_value(
    deps: &Deps,
    positions: &Positions,
) -> ContractResult<CollateralValue> {
    let deposits = calculate_deposits_value(deps, &positions.deposits)?;
    let vaults = calculate_vaults_value(deps, &positions.vaults)?;

    Ok(CollateralValue {
        total_collateral_value: deposits
            .total_collateral_value
            .checked_add(vaults.total_collateral_value)?,
        max_ltv_adjusted_collateral: deposits
            .max_ltv_adjusted_collateral
            .checked_add(vaults.max_ltv_adjusted_collateral)?,
        liquidation_threshold_adjusted_collateral: deposits
            .liquidation_threshold_adjusted_collateral
            .checked_add(vaults.liquidation_threshold_adjusted_collateral)?,
    })
}

fn calculate_vaults_value(
    deps: &Deps,
    vaults: &[VaultPosition],
) -> ContractResult<CollateralValue> {
    let oracle = ORACLE.load(deps.storage)?;
    let red_bank = RED_BANK.load(deps.storage)?;

    let mut total_collateral_value = Uint128::zero();
    let mut max_ltv_adjusted_collateral = Uint128::zero();
    let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

    for v in vaults {
        // Unlocked & locked denominated in vault coins
        let vault_coin_amount = v.amount.unlocked().checked_add(v.amount.locked())?;
        let vault_coin_value = v
            .vault
            .query_value(&deps.querier, &oracle, vault_coin_amount)?;
        total_collateral_value = total_collateral_value.checked_add(vault_coin_value)?;

        let config = VAULT_CONFIGS.load(deps.storage, &v.vault.address)?;
        max_ltv_adjusted_collateral = vault_coin_value
            .checked_mul_floor(config.max_ltv)?
            .checked_add(max_ltv_adjusted_collateral)?;
        liquidation_threshold_adjusted_collateral = vault_coin_value
            .checked_mul_floor(config.liquidation_threshold)?
            .checked_add(liquidation_threshold_adjusted_collateral)?;

        // Unlocking positions denominated in underlying token
        let info = v.vault.query_info(&deps.querier)?;
        let PriceResponse { price, .. } = oracle.query_price(&deps.querier, &info.base_token)?;
        let Market {
            max_loan_to_value,
            liquidation_threshold,
            ..
        } = red_bank.query_market(&deps.querier, &info.base_token)?;
        for u in v.amount.unlocking().positions() {
            let underlying_value = u.coin.amount.checked_mul_floor(price)?;
            total_collateral_value = total_collateral_value.checked_add(underlying_value)?;
            max_ltv_adjusted_collateral = underlying_value
                .checked_mul_floor(max_loan_to_value)?
                .checked_add(max_ltv_adjusted_collateral)?;
            liquidation_threshold_adjusted_collateral = underlying_value
                .checked_mul_floor(liquidation_threshold)?
                .checked_add(liquidation_threshold_adjusted_collateral)?;
        }
    }

    Ok(CollateralValue {
        total_collateral_value,
        max_ltv_adjusted_collateral,
        liquidation_threshold_adjusted_collateral,
    })
}

fn calculate_deposits_value(deps: &Deps, deposits: &[Coin]) -> ContractResult<CollateralValue> {
    let oracle = ORACLE.load(deps.storage)?;
    let red_bank = RED_BANK.load(deps.storage)?;

    let mut total_collateral_value = Uint128::zero();
    let mut max_ltv_adjusted_collateral = Uint128::zero();
    let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

    for c in deposits {
        let value = oracle.query_value(&deps.querier, c)?;
        total_collateral_value = total_collateral_value.checked_add(value)?;

        let Market {
            max_loan_to_value,
            liquidation_threshold,
            ..
        } = red_bank.query_market(&deps.querier, &c.denom)?;

        // If coin has been de-listed, drop MaxLTV to zero
        let max_ltv = if ALLOWED_COINS.contains(deps.storage, &c.denom) {
            max_loan_to_value
        } else {
            Decimal::zero()
        };
        let max_ltv_adjusted = value.checked_mul_floor(max_ltv)?;
        max_ltv_adjusted_collateral = max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

        let liq_adjusted = value.checked_mul_floor(liquidation_threshold)?;
        liquidation_threshold_adjusted_collateral =
            liquidation_threshold_adjusted_collateral.checked_add(liq_adjusted)?;
    }
    Ok(CollateralValue {
        total_collateral_value,
        max_ltv_adjusted_collateral,
        liquidation_threshold_adjusted_collateral,
    })
}

fn calculate_total_debt_value(deps: &Deps, debts: &[DebtAmount]) -> ContractResult<Uint128> {
    let oracle = ORACLE.load(deps.storage)?;
    let mut total = Uint128::zero();
    for debt in debts {
        let debt_value = oracle.query_value(
            &deps.querier,
            &Coin {
                denom: debt.denom.clone(),
                amount: debt.amount,
            },
        )?;
        total = total.checked_add(debt_value)?;
    }
    Ok(total)
}

pub fn assert_below_max_ltv(deps: Deps, env: Env, account_id: &str) -> ContractResult<Response> {
    let health = compute_health(deps, &env, account_id)?;

    if health.is_above_max_ltv() {
        return Err(ContractError::AboveMaxLTV {
            account_id: account_id.to_string(),
            max_ltv_health_factor: val_or_na(health.max_ltv_health_factor),
        });
    }

    let event = Event::new("position_changed")
        .add_attribute("timestamp", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("account_id", account_id)
        .add_attribute("assets_value", health.total_collateral_value.to_string())
        .add_attribute("debts_value", health.total_debt_value.to_string())
        .add_attribute(
            "lqdt_health_factor",
            val_or_na(health.liquidation_health_factor),
        )
        .add_attribute("liquidatable", health.is_liquidatable().to_string())
        .add_attribute(
            "max_ltv_health_factor",
            val_or_na(health.max_ltv_health_factor),
        )
        .add_attribute("above_max_ltv", health.is_above_max_ltv().to_string());

    Ok(Response::new()
        .add_attribute("action", "rover/credit-manager/callback/assert_health")
        .add_event(event))
}

pub fn val_or_na(opt: Option<Decimal>) -> String {
    opt.map_or_else(|| "n/a".to_string(), |dec| dec.to_string())
}
