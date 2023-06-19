use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, SubMsgResponse};
use mars_v3_zapper_base::{
    error::{ContractError::ReplyError, ContractResult},
    msg::NewPositionRequest,
    traits::PositionManager,
};
use osmosis_std::types::{
    cosmos::base::v1beta1,
    osmosis::concentratedliquidity::v1beta1::{MsgCreatePosition, MsgCreatePositionResponse},
};

pub struct OsmosisPositionManager {}

impl PositionManager for OsmosisPositionManager {
    fn create_new_position(
        _: DepsMut,
        env: Env,
        p: NewPositionRequest,
    ) -> ContractResult<CosmosMsg> {
        let create_msg = MsgCreatePosition {
            pool_id: p.pool_id,
            sender: env.contract.address.to_string(),
            lower_tick: p.lower_tick,
            upper_tick: p.upper_tick,
            token_min_amount0: p.token_min_amount0,
            token_min_amount1: p.token_min_amount1,
            tokens_provided: p.tokens_provided.to_v1beta_coins(),
        };
        Ok(create_msg.into())
    }

    fn parse_position_id(_: DepsMut, _: Env, response: SubMsgResponse) -> ContractResult<String> {
        let Some(b) = response.data else {
            return Err(ReplyError("No data sent back after creating position".to_string()))
        };

        let parsed_response: MsgCreatePositionResponse = b.try_into()?;
        Ok(parsed_response.position_id.to_string())
    }
}

pub trait ToV1BetaCoins {
    fn to_v1beta_coins(&self) -> Vec<v1beta1::Coin>;
}

impl ToV1BetaCoins for Vec<Coin> {
    fn to_v1beta_coins(&self) -> Vec<v1beta1::Coin> {
        self.iter()
            .map(|c| v1beta1::Coin {
                denom: c.denom.clone(),
                amount: c.amount.to_string(),
            })
            .collect()
    }
}
