use std::str::FromStr;

use cosmwasm_std::Coin;
use osmosis_std::types::cosmos::{
    bank::v1beta1::{
        MsgSend, MsgSendResponse, QueryAllBalancesRequest, QueryAllBalancesResponse,
        QueryBalanceRequest, QueryBalanceResponse, QueryTotalSupplyRequest,
        QueryTotalSupplyResponse,
    },
    base::v1beta1::Coin as CosmosCoin,
};
use osmosis_test_tube::{
    fn_execute, fn_query, Account, Module, Runner, RunnerExecuteResult, SigningAccount,
};

pub struct Bank<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Bank<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self {
            runner,
        }
    }
}

impl<'a, R> Bank<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub _send: MsgSend => MsgSendResponse
    }

    fn_query! {
        pub _query_balance ["/cosmos.bank.v1beta1.Query/Balance"]: QueryBalanceRequest => QueryBalanceResponse
    }

    fn_query! {
        pub query_all_balances ["/cosmos.bank.v1beta1.Query/AllBalances"]: QueryAllBalancesRequest => QueryAllBalancesResponse
    }

    fn_query! {
        pub query_total_supply ["/cosmos.bank.v1beta1.Query/TotalSupply"]: QueryTotalSupplyRequest => QueryTotalSupplyResponse
    }

    pub fn send(
        &self,
        sender: &SigningAccount,
        to_addr: &str,
        coins: &[Coin],
    ) -> RunnerExecuteResult<MsgSendResponse> {
        self._send(
            MsgSend {
                from_address: sender.address(),
                to_address: to_addr.to_string(),
                amount: coins
                    .iter()
                    .map(|c| CosmosCoin {
                        denom: c.denom.clone(),
                        amount: c.amount.to_string(),
                    })
                    .collect(),
            },
            sender,
        )
    }

    pub fn query_balance(&self, addr: &str, denom: &str) -> u128 {
        self._query_balance(&QueryBalanceRequest {
            address: addr.to_string(),
            denom: denom.to_string(),
        })
        .unwrap()
        .balance
        .map(|c| u128::from_str(&c.amount).unwrap())
        .unwrap_or(0)
    }
}
