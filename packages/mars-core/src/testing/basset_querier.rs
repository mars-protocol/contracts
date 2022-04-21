use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult};

use basset::hub::{QueryMsg, StateResponse};

#[derive(Clone, Default)]
pub struct BAssetQuerier {
    pub state_response: Option<StateResponse>,
}

impl BAssetQuerier {
    pub fn handle_query(&self, request: &QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match &request {
            QueryMsg::State {} => match self.state_response.as_ref() {
                Some(resp) => to_binary(resp).into(),
                None => panic!("[mock]: StateResponse is not provided for query"),
            },
            _ => Err("[mock]: Unsupported basset query").into(),
        };

        Ok(ret).into()
    }
}
