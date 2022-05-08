use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};

use stader::msg::{QueryMsg, QueryStateResponse};

#[derive(Clone, Default)]
pub struct StaderQuerier {
    pub state_response: Option<QueryStateResponse>,
}

impl StaderQuerier {
    pub fn handle_query(&self, request: &QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match &request {
            QueryMsg::State {} => match self.state_response.as_ref() {
                Some(resp) => to_binary(resp).into(),
                None => panic!("[mock]: StateResponse is not provided for query"),
            },
            _ => Err("[mock]: Unsupported stader query").into(),
        };

        Ok(ret).into()
    }
}
