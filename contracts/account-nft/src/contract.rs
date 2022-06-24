use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw721::NumTokensResponse;
use cw721_base::{
    ContractError, Cw721Contract, ExecuteMsg, Extension, InstantiateMsg, MintMsg, QueryMsg,
};

// Extending CW721 base contract
pub type Parent<'a> = Cw721Contract<'a, Extension, Empty>;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Parent::default().instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<Extension>,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint(mint_msg) => mint_override(deps, env, info, mint_msg),
        _ => Parent::default().execute(deps, env, info, msg),
    }
}

fn mint_override(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MintMsg<Extension>,
) -> Result<Response, ContractError> {
    let parent = Parent::default();

    let num_tokens: NumTokensResponse =
        deps.querier.query_wasm_smart(&env.contract.address, &QueryMsg::NumTokens {})?;

    let mint_msg_override = MintMsg {
        token_id: (num_tokens.count + 1).to_string(),
        owner: msg.owner,
        token_uri: None,
        extension: None,
    };

    parent.mint(deps, env, info, mint_msg_override)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Parent::default().query(deps, env, msg)
}
