use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult};
use cw721::TokensResponse;
use mars_account_nft_types::msg::QueryMsg;

#[cw_serde]
pub struct AccountNftBase<T>(T);

impl<T> AccountNftBase<T> {
    pub fn new(address: T) -> AccountNftBase<T> {
        AccountNftBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type AccountNftUnchecked = AccountNftBase<String>;
pub type AccountNft = AccountNftBase<Addr>;

impl From<AccountNft> for AccountNftUnchecked {
    fn from(account_nft: AccountNft) -> Self {
        Self(account_nft.0.to_string())
    }
}

impl AccountNftUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<AccountNft> {
        Ok(AccountNftBase(api.addr_validate(self.address())?))
    }
}

impl AccountNft {
    pub fn query_next_id(&self, querier: &QuerierWrapper) -> StdResult<String> {
        querier.query_wasm_smart(self.address().to_string(), &QueryMsg::NextId {})
    }

    pub fn query_tokens(
        &self,
        querier: &QuerierWrapper,
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
        )
    }
}
