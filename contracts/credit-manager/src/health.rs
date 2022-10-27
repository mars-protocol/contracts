use cosmwasm_std::{Coin, Decimal, Deps, Env, Event, Response};
use mars_health::health::{Health, Position};
use mars_health::query::MarsQuerier;

use rover::adapters::vault::{Total, VaultPosition};
use rover::adapters::{Oracle, RedBank};
use rover::error::{ContractError, ContractResult};
use rover::traits::{Coins, IntoDecimal};

use crate::query::query_positions;
use crate::state::{ORACLE, RED_BANK, VAULT_CONFIGS};

// Given Red Bank and Mars-Oracle does not have knowledge of vaults,
// we cannot use Health::compute_health_from_coins() and must assemble positions manually
pub fn compute_health(deps: Deps, env: &Env, account_id: &str) -> ContractResult<Health> {
    let oracle = ORACLE.load(deps.storage)?;
    let red_bank = RED_BANK.load(deps.storage)?;

    let res = query_positions(deps, env, account_id)?;

    let mut positions: Vec<Position> = vec![];
    let coin_positions =
        get_positions_for_coins(&deps, &res.coins, &res.debts.to_coins(), &oracle, &red_bank)?;
    positions.extend(coin_positions);
    let vault_positions = get_positions_for_vaults(&deps, &res.vaults, &oracle)?;
    positions.extend(vault_positions);

    let health = Health::compute_health(&positions)?;
    Ok(health)
}

fn get_positions_for_coins(
    deps: &Deps,
    collateral: &[Coin],
    debt: &[Coin],
    oracle: &Oracle,
    red_bank: &RedBank,
) -> ContractResult<Vec<Position>> {
    let querier = MarsQuerier::new(&deps.querier, oracle.address(), red_bank.address());
    let positions = Health::positions_from_coins(&querier, collateral, debt)?
        .into_values()
        .collect();
    Ok(positions)
}

fn get_positions_for_vaults(
    deps: &Deps,
    vaults: &[VaultPosition],
    oracle: &Oracle,
) -> ContractResult<Vec<Position>> {
    vaults
        .iter()
        .map(|v| {
            let info = v.vault.query_info(&deps.querier)?;
            let query_res = oracle.query_price(&deps.querier, &info.token_denom)?;
            let config = VAULT_CONFIGS.load(deps.storage, &v.vault.address)?;
            Ok(Position {
                denom: query_res.denom,
                price: query_res.price,
                collateral_amount: v.amount.total().to_dec()?,
                debt_amount: Decimal::zero(),
                max_ltv: config.max_ltv,
                liquidation_threshold: config.liquidation_threshold,
            })
        })
        .collect()
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
        .add_attribute("action", "rover/credit_manager/callback/assert_health")
        .add_event(event))
}

pub fn val_or_na(opt: Option<Decimal>) -> String {
    opt.map_or_else(|| "n/a".to_string(), |dec| dec.to_string())
}
