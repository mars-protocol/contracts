use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;

use crate::{
    error::{ContractError, ContractError::Unauthorized, ContractResult},
    msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, NewPositionRequest, QueryMsg},
    state::OWNER,
    traits::{OptionFilter, PositionManager},
    utils::assert_exact_funds_sent,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const V3_POSITION_CREATED_EVENT_TYPE: &str = "v3_position_created";
pub const V3_POSITION_ATTR_KEY: &str = "position_id";
pub const REFUND_EVENT_TYPE: &str = "execute_callback_refund_coin";
pub const REFUND_AMOUNT_ATTR_KEY: &str = "coins_returned";
pub const REFUND_RECIPIENT_ATTR_KEY: &str = "recipient";

pub const CREATE_POSITION_REPLY_ID: u64 = 1001;

pub struct V3ZapperBase<M>
where
    M: PositionManager,
{
    pub position_manager: PhantomData<M>,
}

impl<M> Default for V3ZapperBase<M>
where
    M: PositionManager,
{
    fn default() -> Self {
        Self {
            position_manager: PhantomData,
        }
    }
}

impl<M> V3ZapperBase<M>
where
    M: PositionManager,
{
    pub fn instantiate(&self, deps: DepsMut, msg: InstantiateMsg) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

        OWNER.initialize(
            deps.storage,
            deps.api,
            SetInitialOwner {
                owner: msg.owner,
            },
        )?;

        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::CreatePosition(p) => self.create_position(deps, env, info, p),
            ExecuteMsg::UpdateOwner(update) => Ok(OWNER.update(deps, info, update)?),
            ExecuteMsg::Callback(msg) => self.handle_callback(deps, env, info, msg),
        }
    }

    fn handle_callback(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: CallbackMsg,
    ) -> ContractResult<Response> {
        if info.sender != env.contract.address {
            return Err(Unauthorized);
        }
        match msg {
            CallbackMsg::RefundCoin {
                recipient,
                denoms,
            } => self.refund_coin(deps, env, recipient, &denoms),
        }
    }

    pub fn query(&self, deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Owner {} => to_binary(&OWNER.query(deps.storage)?),
        }
    }

    pub fn reply(&self, deps: DepsMut, env: Env, reply: Reply) -> ContractResult<Response> {
        let response = reply.result.into_result().map_err(StdError::generic_err)?;
        let position_id = match reply.id {
            CREATE_POSITION_REPLY_ID => M::parse_position_id(deps, env, response)?,
            id => return Err(ContractError::ReplyError(format!("reply id {id} is not valid"))),
        };
        let event = Event::new(V3_POSITION_CREATED_EVENT_TYPE.to_string())
            .add_attribute(V3_POSITION_ATTR_KEY.to_string(), position_id);
        Ok(Response::new().add_event(event))
    }

    pub fn create_position(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        request: NewPositionRequest,
    ) -> ContractResult<Response> {
        OWNER.assert_owner(deps.storage, &info.sender)?;

        let request_coins = vec![&request.token_desired0, &request.token_desired1].only_some();
        assert_exact_funds_sent(&info, &request_coins)?;

        // Creating positions do not guarantee all funds will be used. Refund the leftovers.
        let refund_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::RefundCoin {
                recipient: info.sender,
                denoms: request_coins.iter().map(|c| c.denom.clone()).collect(),
            }))?,
        });

        let create_msg = M::create_new_position(deps, env, request)?;
        let create_submsg = SubMsg::reply_on_success(create_msg, CREATE_POSITION_REPLY_ID);

        Ok(Response::new().add_submessage(create_submsg).add_message(refund_msg))
    }

    pub fn refund_coin(
        &self,
        deps: DepsMut,
        env: Env,
        recipient: Addr,
        denoms: &[String],
    ) -> ContractResult<Response> {
        let mut coins_to_return = vec![];
        for denom in denoms {
            let balance = deps.querier.query_balance(env.contract.address.clone(), denom)?;
            if !balance.amount.is_zero() {
                coins_to_return.push(balance)
            }
        }

        let coins_refunded =
            coins_to_return.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");

        let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: coins_to_return,
        });

        let event = Event::new(REFUND_EVENT_TYPE)
            .add_attribute(REFUND_AMOUNT_ATTR_KEY, coins_refunded)
            .add_attribute(REFUND_RECIPIENT_ATTR_KEY, recipient);

        Ok(Response::new().add_message(transfer_msg).add_event(event))
    }
}
