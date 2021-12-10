use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult};

use crate::math::decimal::Decimal;
use crate::staking::msg::QueryMsg;

pub struct StakingQuerier {
    pub xmars_per_mars: Decimal,
    pub mars_per_xmars: Decimal,
}

impl Default for StakingQuerier {
    fn default() -> Self {
        StakingQuerier {
            xmars_per_mars: Decimal::one(),
            mars_per_xmars: Decimal::one(),
        }
    }
}

impl StakingQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
        let staking = Addr::unchecked("staking");
        if *contract_addr != staking {
            panic!(
                "[mock]: Staking request made to {} shoud be {}",
                contract_addr, staking
            );
        }

        let ret: ContractResult<Binary> = match query {
            QueryMsg::XMarsPerMars {} => to_binary(&self.xmars_per_mars).into(),
            QueryMsg::MarsPerXMars {} => to_binary(&self.mars_per_xmars).into(),
            _ => Err("[mock]: Unsupported staking query").into(),
        };

        Ok(ret).into()
    }
}
