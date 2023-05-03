use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, SubMsgResponse};

use crate::{error::ContractResult, msg::NewPositionRequest};

pub trait PositionManager {
    /// Owner should be the zapper contract itself (not the user)
    fn create_new_position(
        deps: DepsMut,
        env: Env,
        request: NewPositionRequest,
    ) -> ContractResult<CosmosMsg>;

    /// Responsible for parsing the reply message to acquire the id of
    /// the newly created position
    fn parse_position_id(
        deps: DepsMut,
        env: Env,
        response: SubMsgResponse,
    ) -> ContractResult<String>;
}

pub trait OptionFilter<T> {
    fn only_some(&self) -> Vec<T>;
}

impl OptionFilter<Coin> for Vec<&Option<Coin>> {
    fn only_some(&self) -> Vec<Coin> {
        self.iter().filter_map(|x| x.as_ref()).cloned().collect()
    }
}
