use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use mars_red_bank_types::red_bank::Market;

#[cw_serde]
pub struct CoinInfo {
    pub price: Decimal,
    pub market: Market,
}

pub fn umars_info() -> CoinInfo {
    CoinInfo {
        price: Decimal::from_atomics(1u128, 0).unwrap(),
        market: Market {
            denom: "umars".to_string(),
            max_loan_to_value: Decimal::from_atomics(8u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(84u128, 2).unwrap(),
            liquidation_bonus: Decimal::from_atomics(12u128, 2).unwrap(),
            ..Default::default()
        },
    }
}

pub fn udai_info() -> CoinInfo {
    CoinInfo {
        price: Decimal::from_atomics(313451u128, 6).unwrap(),
        market: Market {
            denom: "udai".to_string(),
            max_loan_to_value: Decimal::from_atomics(85u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
            liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
            ..Default::default()
        },
    }
}

pub fn uluna_info() -> CoinInfo {
    CoinInfo {
        price: Decimal::from_atomics(100u128, 1).unwrap(),
        market: Market {
            denom: "uluna".to_string(),
            max_loan_to_value: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
            ..Default::default()
        },
    }
}

pub fn ustars_info() -> CoinInfo {
    CoinInfo {
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        market: Market {
            denom: "ustars".to_string(),
            max_loan_to_value: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
            ..Default::default()
        },
    }
}

pub fn ujuno_info() -> CoinInfo {
    CoinInfo {
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        market: Market {
            denom: "ujuno".to_string(),
            max_loan_to_value: Decimal::from_atomics(8u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
            liquidation_bonus: Decimal::from_atomics(12u128, 2).unwrap(),
            ..Default::default()
        },
    }
}
