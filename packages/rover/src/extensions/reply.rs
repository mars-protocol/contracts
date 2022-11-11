use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Reply, StdError, StdResult, SubMsgResult};
use cosmwasm_vault_standard::extensions::lockup::{
    UNLOCKING_POSITION_ATTR_KEY, UNLOCKING_POSITION_CREATED_EVENT_TYPE,
};

#[cw_serde]
pub struct AssetTransferMsg {
    pub recipient: String,
    pub sender: String,
    pub amount: Vec<Coin>,
}

#[cw_serde]
pub struct UnlockEvent {
    pub id: u64,
}

pub trait AttrParse {
    fn parse_unlock_event(self) -> StdResult<UnlockEvent>;
}

impl AttrParse for Reply {
    fn parse_unlock_event(self) -> StdResult<UnlockEvent> {
        match self.result {
            SubMsgResult::Err(err) => Err(StdError::generic_err(err)),
            SubMsgResult::Ok(response) => {
                let unlock_event = response
                    .events
                    .iter()
                    .find(|event| {
                        event.ty == format!("wasm-{}", UNLOCKING_POSITION_CREATED_EVENT_TYPE)
                    })
                    .ok_or_else(|| StdError::generic_err("No unlock event"))?;

                let id = &unlock_event
                    .attributes
                    .iter()
                    .find(|x| x.key == UNLOCKING_POSITION_ATTR_KEY)
                    .ok_or_else(|| StdError::generic_err("No id attribute"))?
                    .value;

                Ok(UnlockEvent {
                    id: id
                        .parse::<u64>()
                        .map_err(|_| StdError::generic_err("Could not parse id from reply"))?,
                })
            }
        }
    }
}
