use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdResult, Uint128,
};
use cw_utils::one_coin;

use crate::{CallbackMsg, ContractError, ExecuteMsg, InstantiateMsg, LpPool, QueryMsg};

pub struct ZapperBase<P>
where
    P: LpPool,
{
    /// Phantom data holds generics
    pub custom_pool: PhantomData<P>,
}

impl<P> Default for ZapperBase<P>
where
    P: LpPool,
{
    fn default() -> Self {
        Self {
            custom_pool: PhantomData,
        }
    }
}

impl<P> ZapperBase<P>
where
    P: LpPool,
{
    pub fn instantiate(
        &self,
        _deps: DepsMut,
        _msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            ExecuteMsg::ProvideLiquidity {
                lp_token_out,
                recipient,
                minimum_receive,
            } => Self::execute_provide_liquidity(
                deps,
                env,
                info,
                lp_token_out,
                recipient,
                minimum_receive,
            ),
            ExecuteMsg::WithdrawLiquidity { recipient } => {
                Self::execute_withdraw_liquidity(deps, env, info, recipient)
            }
            ExecuteMsg::Callback(msg) => {
                // Can only be called by the contract itself
                if info.sender != env.contract.address {
                    return Err(ContractError::Unauthorized {});
                }
                match msg {
                    CallbackMsg::ReturnCoin {
                        balance_before,
                        recipient,
                    } => Self::execute_return_tokens(deps, env, info, balance_before, recipient),
                }
            }
        }
    }

    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::EstimateProvideLiquidity {
                lp_token_out,
                coins_in,
            } => Self::query_estimate_provide_liquidity(deps, env, lp_token_out, coins_in),
            QueryMsg::EstimateWithdrawLiquidity { coin_in } => {
                Self::query_estimate_withdraw_liquidity(deps, env, coin_in)
            }
        }
    }

    fn execute_provide_liquidity(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        lp_token_out: String,
        recipient: Option<String>,
        minimum_receive: Uint128,
    ) -> Result<Response, ContractError> {
        let pool = P::get_pool_for_lp_token(deps.as_ref(), &lp_token_out)?;

        // Unwrap recipient or use caller's address
        let recipient = recipient.map_or(Ok(info.sender), |x| deps.api.addr_validate(&x))?;

        let response = pool.provide_liquidity(
            deps.as_ref(),
            &env,
            info.funds.clone().into(),
            minimum_receive,
        )?;

        // Query current contract coin balances
        let mut coin_balances: Vec<Coin> = Vec::with_capacity(info.funds.len() + 1); // funds + lp token
        for funded_coin in info.funds {
            let mut coin_balance = deps
                .querier
                .query_balance(&env.contract.address, &funded_coin.denom)?;
            coin_balance.amount = coin_balance.amount.checked_sub(funded_coin.amount)?;
            coin_balances.push(coin_balance);
        }

        // Query current contract LP token balance
        let lp_token_balance = deps
            .querier
            .query_balance(&env.contract.address, &lp_token_out)?;
        coin_balances.push(lp_token_balance);

        // Callbacks to return remaining coins and LP tokens
        let callback_msgs = prepare_return_coin_callbacks(&env, recipient.clone(), coin_balances)?;

        let event = Event::new("rover/zapper/execute_provide_liquidity")
            .add_attribute("lp_token_out", lp_token_out)
            .add_attribute("minimum_receive", minimum_receive)
            .add_attribute("recipient", recipient);

        Ok(response.add_messages(callback_msgs).add_event(event))
    }

    fn execute_withdraw_liquidity(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: Option<String>,
    ) -> Result<Response, ContractError> {
        // Make sure only one coin is sent
        one_coin(&info)?;

        let lp_token = info.funds[0].clone();
        let pool = P::get_pool_for_lp_token(deps.as_ref(), &lp_token.denom)?;

        // Unwrap recipient or use caller
        let recipient = recipient.map_or(Ok(info.sender), |x| deps.api.addr_validate(&x))?;

        // Use returned coins to check what denoms should be received
        let coins_returned =
            pool.simulate_withdraw_liquidity(deps.as_ref(), &lp_token.clone().into())?;
        let coins_returned_str = coins_returned.to_string();

        let response = pool.withdraw_liquidity(deps.as_ref(), &env, lp_token.clone().into())?;

        // Query current contract coin balances
        let mut coin_balances: Vec<Coin> = Vec::with_capacity(coins_returned.len() + 1); // coins returned + lp token
        for coin_returned in coins_returned.to_vec() {
            let coin_returned: Coin = coin_returned.try_into()?;
            let coin_balance = deps
                .querier
                .query_balance(&env.contract.address, coin_returned.denom)?;
            coin_balances.push(coin_balance);
        }

        // Query current contract LP token balance
        let mut lp_token_balance = deps
            .querier
            .query_balance(&env.contract.address, &lp_token.denom)?;
        lp_token_balance.amount = lp_token_balance.amount.checked_sub(lp_token.amount)?;
        coin_balances.push(lp_token_balance);

        // Callbacks to return remaining coins and LP tokens
        let callback_msgs = prepare_return_coin_callbacks(&env, recipient.clone(), coin_balances)?;

        let event = Event::new("rover/zapper/execute_withdraw_liquidity")
            .add_attribute("lp_token", lp_token.denom)
            .add_attribute("coins_returned", coins_returned_str)
            .add_attribute("recipient", recipient);

        Ok(response.add_messages(callback_msgs).add_event(event))
    }

    fn execute_return_tokens(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        balance_before: Coin,
        recipient: Addr,
    ) -> Result<Response, ContractError> {
        let balance_after = deps
            .querier
            .query_balance(env.contract.address, &balance_before.denom)?;
        let return_amount = balance_after.amount.checked_sub(balance_before.amount)?;

        if return_amount.is_zero() {
            return Ok(Response::new());
        }

        let return_coin = Coin {
            denom: balance_before.denom,
            amount: return_amount,
        };
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![return_coin.clone()],
        });

        let event = Event::new("rover/zapper/execute_callback_return_lp_tokens")
            .add_attribute("coin_returned", return_coin.to_string())
            .add_attribute("recipient", recipient);

        Ok(Response::new().add_message(send_msg).add_event(event))
    }

    fn query_estimate_provide_liquidity(
        deps: Deps,
        env: Env,
        lp_token_out: String,
        coins_in: Vec<Coin>,
    ) -> StdResult<Binary> {
        let pool = P::get_pool_for_lp_token(deps, &lp_token_out)?;

        let lp_tokens_returned = pool.simulate_provide_liquidity(deps, &env, coins_in.into())?;

        to_binary(&lp_tokens_returned.amount)
    }

    fn query_estimate_withdraw_liquidity(
        deps: Deps,
        _env: Env,
        coin_in: Coin,
    ) -> StdResult<Binary> {
        let pool = P::get_pool_for_lp_token(deps, &coin_in.denom)?;

        let coins_returned = pool.simulate_withdraw_liquidity(deps, &coin_in.into())?;

        let native_coins_returned: Vec<Coin> = coins_returned
            .to_vec()
            .into_iter()
            .filter_map(|x| x.try_into().ok()) // filter out non native coins
            .collect();

        to_binary(&native_coins_returned)
    }
}

fn prepare_return_coin_callbacks(
    env: &Env,
    recipient: Addr,
    coin_balances: Vec<Coin>,
) -> StdResult<Vec<CosmosMsg>> {
    coin_balances
        .into_iter()
        .map(|coin_balance| {
            CallbackMsg::ReturnCoin {
                balance_before: coin_balance,
                recipient: recipient.clone(),
            }
            .into_cosmos_msg(env)
        })
        .collect()
}
