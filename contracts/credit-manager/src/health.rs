use cosmwasm_std::{Coin, Decimal, Deps, Env, Event, Response};
use mars_health::health::{Health, Position};
use mars_health::query::MarsQuerier;
use mars_outpost::red_bank::Market;

use rover::adapters::vault::VaultPosition;
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
    let positions = vaults
        .iter()
        .map(|v| {
            let info = v.vault.query_info(&deps.querier)?;
            let price_res = oracle.query_price(&deps.querier, &info.vault_token)?;
            let config = VAULT_CONFIGS.load(deps.storage, &v.vault.address)?;
            let mut positions = vec![];

            positions.push(Position {
                denom: price_res.denom,
                price: price_res.price,
                collateral_amount: v
                    .amount
                    .unlocked()
                    .checked_add(v.amount.locked())?
                    .to_dec()?,
                debt_amount: Decimal::zero(),
                max_ltv: config.max_ltv,
                liquidation_threshold: config.liquidation_threshold,
            });

            let red_bank = RED_BANK.load(deps.storage)?;
            for u in v.amount.unlocking().positions() {
                let price_res = oracle.query_price(&deps.querier, &u.coin.denom)?;
                let Market {
                    max_loan_to_value,
                    liquidation_threshold,
                    ..
                } = red_bank.query_market(&deps.querier, &u.coin.denom)?;
                positions.push(Position {
                    denom: price_res.denom,
                    price: price_res.price,
                    collateral_amount: u.coin.amount.to_dec()?,
                    debt_amount: Decimal::zero(),
                    max_ltv: max_loan_to_value,
                    liquidation_threshold,
                })
            }

            Ok(positions)
        })
        .collect::<ContractResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(positions)
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
