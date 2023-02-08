use cosmwasm_std::{coins, BankMsg, DepsMut, Response};

use super::assert_version;
use crate::{
    error::ContractError,
    execute::{CONTRACT_NAME, CONTRACT_VERSION},
};

const FROM_VERSION: &str = "1.0.0";

// in this tx:
// https://www.mintscan.io/osmosis/txs/2F214EE3A22DC93E61DE9A49BE616B317EB28AFC5E43B0AF07800AC7E6435522
//
// the user sent 10,001 axlUSDC to the Red Bank contract by mistake.
// here we refund it
const USER: &str = "osmo1xll4488tnahcx8dsvtkcljvvzclmy4ca4nx986";
const DENOM: &str = "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858";
const AMOUNT: u128 = 10_001_000_000;

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_version(deps.as_ref().storage, CONTRACT_NAME, FROM_VERSION)?;

    // update contract version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // refund the USDC sent to the contract by mistake
    let msg = BankMsg::Send {
        to_address: USER.into(),
        amount: coins(AMOUNT, DENOM),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_dependencies, SubMsg};

    use super::*;

    #[test]
    fn proper_migration() {
        let mut deps = mock_dependencies();

        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION).unwrap();

        let res = migrate(deps.as_mut()).unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0],
            SubMsg::new(BankMsg::Send {
                to_address: USER.into(),
                amount: coins(AMOUNT, DENOM)
            }),
        );
    }
}
