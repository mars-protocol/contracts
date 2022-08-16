use crate::{error::MarsHealthResult, query::MarsQuerier};
use cosmwasm_std::{Coin, Decimal};
use mars_outpost::red_bank::Market;

#[derive(Default, Debug, Clone)]
pub struct Asset {
    #[allow(dead_code)]
    denom: String,
    price: Decimal,
    amount: Decimal,
    max_ltv: Decimal,
    liq_threshold: Decimal,
}

impl Asset {
    #[inline]
    pub fn value(&self) -> MarsHealthResult<Decimal> {
        Ok(self.amount.checked_mul(self.price)?)
    }

    #[inline]
    pub fn value_max_ltv_adjusted(&self) -> MarsHealthResult<Decimal> {
        Ok(self.value()?.checked_mul(self.max_ltv)?)
    }

    #[inline]
    pub fn value_liq_threshold_adjusted(&self) -> MarsHealthResult<Decimal> {
        Ok(self.value()?.checked_mul(self.liq_threshold)?)
    }

    fn try_from_coin(querier: &MarsQuerier, coin: &Coin) -> MarsHealthResult<Self> {
        let Market {
            max_loan_to_value,
            liquidation_threshold,
            ..
        } = querier.query_market(&coin.denom)?;

        Ok(Asset {
            denom: coin.denom.clone(),
            price: querier.query_price(&coin.denom)?,
            amount: Decimal::from_atomics(coin.amount, 0)?,
            max_ltv: max_loan_to_value,
            liq_threshold: liquidation_threshold,
        })
    }

    pub fn try_assets_from_coins(
        querier: &MarsQuerier,
        coins: &[Coin],
    ) -> MarsHealthResult<Vec<Asset>> {
        coins.iter().map(|coin| Asset::try_from_coin(querier, coin)).collect::<Result<Vec<_>, _>>()
    }
}
