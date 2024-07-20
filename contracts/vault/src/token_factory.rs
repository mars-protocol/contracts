use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    attr, Addr, CosmosMsg, Deps, DepsMut, Env, Event, Response, StdResult, Uint128,
};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as CoinMsg,
    osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint},
};

#[cw_serde]
/// Representation of a native token created using the Token Factory.
/// The denom of the token will be `factory/{owner}/{subdenom}`.
pub struct TokenFactoryDenom {
    /// Creator and owner of the denom. Only this address can mint and burn
    /// tokens.
    pub owner: String,
    /// The subdenom of the token. All tokens created using the token factory
    /// have the format `factory/{owner}/{subdenom}`.
    pub subdenom: String,
}

impl TokenFactoryDenom {
    pub const fn new(owner: String, subdenom: String) -> Self {
        Self {
            owner,
            subdenom,
        }
    }

    pub fn instantiate(&self) -> StdResult<Response> {
        let init_msg: CosmosMsg = MsgCreateDenom {
            sender: self.owner.clone(),
            subdenom: self.subdenom.clone(),
        }
        .into();

        let init_event =
            Event::new("vault_token/instantiate").add_attribute("denom", self.to_string());

        Ok(Response::new().add_message(init_msg).add_event(init_event))
    }

    pub fn mint(
        &self,
        _deps: DepsMut,
        env: &Env,
        recipient: &Addr,
        amount: Uint128,
    ) -> StdResult<Response> {
        let mint_msg: CosmosMsg = MsgMint {
            amount: Some(CoinMsg {
                denom: self.to_string(),
                amount: amount.to_string(),
            }),
            sender: env.contract.address.to_string(),
            mint_to_address: recipient.to_string(),
        }
        .into();

        let event = Event::new("vault_token/mint").add_attributes(vec![
            attr("denom", self.to_string()),
            attr("amount", amount.to_string()),
            attr("recipient", recipient.to_string()),
        ]);

        Ok(Response::new().add_message(mint_msg).add_event(event))
    }

    pub fn burn(&self, _deps: DepsMut, env: &Env, amount: Uint128) -> StdResult<Response> {
        let burn_msg: CosmosMsg = MsgBurn {
            amount: Some(CoinMsg {
                denom: self.to_string(),
                amount: amount.to_string(),
            }),
            sender: env.contract.address.to_string(),
            burn_from_address: env.contract.address.to_string(),
        }
        .into();

        let event = Event::new("vault_token/burn").add_attributes(vec![
            attr("denom", self.to_string()),
            attr("amount", amount.to_string()),
        ]);

        Ok(Response::new().add_message(burn_msg).add_event(event))
    }

    pub fn query_balance<A: Into<String>>(&self, deps: Deps, address: A) -> StdResult<Uint128> {
        Ok(deps.querier.query_balance(address, self.to_string())?.amount)
    }

    pub fn query_total_supply(&self, deps: Deps) -> StdResult<Uint128> {
        Ok(deps.querier.query_supply(self.to_string())?.amount)
    }
}

impl Display for TokenFactoryDenom {
    /// Returns the full denom of the token, in the format
    /// `factory/{owner}/{subdenom}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "factory/{}/{}", self.owner, self.subdenom)
    }
}
