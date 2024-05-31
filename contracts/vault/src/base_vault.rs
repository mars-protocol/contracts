use cosmwasm_std::{
    attr, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, Event, Response, StdError, StdResult,
    Uint128,
};
use cw_storage_plus::Item;

use crate::{
    error::ContractError, execute::total_base_token_in_account, token_factory::TokenFactoryDenom,
};

pub const DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN: Uint128 = Uint128::new(1_000_000);

pub struct BaseVault<'a> {
    /// The vault token implementation for this vault
    pub vault_token: Item<'a, TokenFactoryDenom>,

    /// The token that is depositable to the vault
    pub base_token: Item<'a, String>,
}

impl Default for BaseVault<'_> {
    fn default() -> Self {
        BaseVault {
            vault_token: Item::new("vault_token"),
            base_token: Item::new("base_token"),
        }
    }
}

impl<'a> BaseVault<'a> {
    pub fn init(
        &self,
        deps: DepsMut,
        base_token: String,
        vault_token: TokenFactoryDenom,
    ) -> StdResult<Response> {
        self.vault_token.save(deps.storage, &vault_token)?;
        self.base_token.save(deps.storage, &base_token)?;

        vault_token.instantiate()
    }

    pub fn send_base_tokens(
        &self,
        deps: DepsMut,
        recipient: &Addr,
        amount: Uint128,
    ) -> StdResult<Response> {
        let event = Event::new("base_vault/send_base_tokens")
            .add_attributes(vec![attr("recipient", recipient), attr("amount", amount)]);

        Ok(Response::new()
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.into(),
                amount: vec![Coin {
                    denom: self.base_token.load(deps.storage)?,
                    amount,
                }],
            }))
            .add_event(event))
    }

    pub fn calculate_vault_tokens(
        &self,
        base_tokens: Uint128,
        total_staked_amount: Uint128,
        vault_token_supply: Uint128,
    ) -> Result<Uint128, StdError> {
        let vault_tokens = if total_staked_amount.is_zero() {
            base_tokens.checked_mul(DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN)?
        } else {
            vault_token_supply.multiply_ratio(base_tokens, total_staked_amount)
        };

        Ok(vault_tokens)
    }

    pub fn calculate_base_tokens(
        &self,
        vault_tokens: Uint128,
        total_staked_amount: Uint128,
        vault_token_supply: Uint128,
    ) -> Result<Uint128, StdError> {
        let base_tokens = if vault_token_supply.is_zero() {
            vault_tokens.checked_div(DEFAULT_VAULT_TOKENS_PER_STAKED_BASE_TOKEN)?
        } else {
            total_staked_amount.multiply_ratio(vault_tokens, vault_token_supply)
        };

        Ok(base_tokens)
    }

    pub fn burn_vault_tokens_for_base_tokens(
        &self,
        deps: DepsMut,
        env: &Env,
        total_staked_amount: Uint128,
        vault_tokens: Uint128,
    ) -> Result<(Uint128, Response), ContractError> {
        let vault_token = self.vault_token.load(deps.storage)?;
        let vault_token_supply = vault_token.query_total_supply(deps.as_ref())?;

        // calculate base tokens based on the given amount of vault tokens
        let base_tokens =
            self.calculate_base_tokens(vault_tokens, total_staked_amount, vault_token_supply)?;

        let event =
            Event::new("base_vault/burn_vault_tokens_for_base_tokens").add_attributes(vec![
                attr("burned_vault_token_amount", vault_tokens),
                attr("received_base_token_amount", base_tokens),
            ]);

        Ok((base_tokens, vault_token.burn(deps, env, vault_tokens)?.add_event(event)))
    }

    pub fn query_total_vault_token_supply(&self, deps: Deps) -> StdResult<Uint128> {
        let vault_token = self.vault_token.load(deps.storage)?;
        vault_token.query_total_supply(deps)
    }

    pub fn query_vault_token_balance(&self, deps: Deps, address: String) -> StdResult<Uint128> {
        let vault_token = self.vault_token.load(deps.storage)?;
        vault_token.query_balance(deps, address)
    }

    pub fn query_simulate_deposit(
        &self,
        deps: Deps,
        amount: Uint128,
    ) -> Result<Uint128, ContractError> {
        let vault_token_supply = self.vault_token.load(deps.storage)?.query_total_supply(deps)?;
        let total_staked_amount = total_base_token_in_account(deps)?;
        Ok(self.calculate_vault_tokens(amount, total_staked_amount, vault_token_supply)?)
    }

    pub fn query_simulate_withdraw(
        &self,
        deps: Deps,
        amount: Uint128,
    ) -> Result<Uint128, ContractError> {
        let vault_token_supply = self.vault_token.load(deps.storage)?.query_total_supply(deps)?;
        let total_staked_amount = total_base_token_in_account(deps)?;
        Ok(self.calculate_base_tokens(amount, total_staked_amount, vault_token_supply)?)
    }

    pub fn query_total_assets(&self, deps: Deps) -> Result<Uint128, ContractError> {
        total_base_token_in_account(deps)
    }
}
