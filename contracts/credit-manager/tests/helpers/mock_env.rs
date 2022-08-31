use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use mock_oracle::msg::{
    CoinPrice, ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
};
use mock_red_bank::msg::QueryMsg::UserAssetDebt;
use mock_red_bank::msg::{
    CoinMarketInfo, InstantiateMsg as RedBankInstantiateMsg, UserAssetDebtResponse,
};
use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::execute::{Action, CallbackMsg};
use rover::msg::instantiate::ConfigUpdates;
use rover::msg::query::{
    CoinBalanceResponseItem, ConfigResponse, DebtShares, HealthResponse,
    PositionsWithValueResponse, SharesResponseItem,
};
use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{
    mock_account_nft_contract, mock_oracle_contract, mock_red_bank_contract, mock_rover_contract,
    AccountToFund, CoinInfo,
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(1_000_000u128);

pub struct MockEnv {
    pub app: BasicApp,
    pub rover: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub owner: Option<Addr>,
    pub allowed_vaults: Option<Vec<String>>,
    pub allowed_coins: Option<Vec<CoinInfo>>,
    pub oracle: Option<OracleBase<Addr>>,
    pub red_bank: Option<RedBankBase<Addr>>,
    pub setup_nft_contract: bool,
    pub setup_nft_contract_owner: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            owner: None,
            allowed_vaults: None,
            allowed_coins: None,
            oracle: None,
            red_bank: None,
            setup_nft_contract: true,
            setup_nft_contract_owner: true,
            accounts_to_fund: vec![],
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_credit_account(
        &mut self,
        token_id: &str,
        sender: &Addr,
        actions: Vec<Action>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateCreditAccount {
                token_id: token_id.to_string(),
                actions,
            },
            send_funds,
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        new_config: ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateConfig { new_config },
            &[],
        )
    }

    pub fn setup_new_nft_contract(&mut self) -> AnyResult<Addr> {
        let nft_contract = setup_nft_contract(&mut self.app, &self.rover.clone());
        propose_new_nft_contract_owner(
            &mut self.app,
            nft_contract.clone(),
            &self.rover.clone(),
            &self.rover.clone(),
        );
        Ok(nft_contract)
    }

    pub fn create_credit_account(&mut self, sender: &Addr) -> AnyResult<String> {
        let res = self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::CreateCreditAccount {},
            &[],
        )?;
        Ok(self.get_token_id(res))
    }

    fn get_token_id(&mut self, res: AppResponse) -> String {
        let attr: Vec<&String> = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .filter(|attr| attr.key == "token_id")
            .map(|attr| &attr.value)
            .collect();

        assert_eq!(attr.len(), 1);
        attr.first().unwrap().to_string()
    }

    pub fn price_change(&mut self, coin: CoinPrice) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked("anyone"),
                Addr::unchecked(config.oracle),
                &OracleExecuteMsg::ChangePrice(coin),
                &[],
            )
            .unwrap();
    }

    pub fn execute_callback(&mut self, sender: &Addr, msg: CallbackMsg) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(msg),
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_position(&self, token_id: &str) -> PositionsWithValueResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::Positions {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_health(&self, token_id: &str) -> HealthResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::Health {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.rover.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_allowed_vaults(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllowedVaults { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_allowed_coins(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllowedCoins { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_all_coin_balances(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<CoinBalanceResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllCoinBalances { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_all_debt_shares(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<SharesResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllDebtShares { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_all_total_debt_shares(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<DebtShares> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalDebtShares { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_total_debt_shares(&self, denom: &str) -> DebtShares {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::TotalDebtShares(denom.to_string()),
            )
            .unwrap()
    }

    pub fn query_red_bank_debt(&self, denom: &str) -> UserAssetDebtResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.red_bank,
                &UserAssetDebt {
                    user_address: self.rover.to_string(),
                    denom: denom.into(),
                },
            )
            .unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let rover = self.get_rover()?;
        self.deploy_nft_contract(&rover);
        self.fund_users();

        Ok(MockEnv {
            app: take(&mut self.app),
            rover,
        })
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    fn fund_users(&mut self) {
        for account in &self.accounts_to_fund {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: account.addr.to_string(),
                    amount: account.funds.clone(),
                }))
                .unwrap();
        }
    }

    fn deploy_nft_contract(&mut self, rover: &Addr) {
        let nft_contract_owner = Addr::unchecked("original_nft_contract_owner");

        if self.setup_nft_contract {
            let nft_contract = setup_nft_contract(&mut self.app, &nft_contract_owner);
            if self.setup_nft_contract_owner {
                propose_new_nft_contract_owner(
                    &mut self.app,
                    nft_contract.clone(),
                    &nft_contract_owner,
                    rover,
                );
                // Update config to save new nft_contract
                self.app
                    .execute_contract(
                        self.get_owner(),
                        rover.clone(),
                        &ExecuteMsg::UpdateConfig {
                            new_config: ConfigUpdates {
                                account_nft: Some(nft_contract.to_string()),
                                ..Default::default()
                            },
                        },
                        &[],
                    )
                    .unwrap();
            }
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Get or defaults
    //--------------------------------------------------------------------------------------------------

    fn get_rover(&mut self) -> AnyResult<Addr> {
        let code_id = self.app.store_code(mock_rover_contract());
        let oracle = self.get_oracle().into();
        let red_bank = self.get_red_bank().into();
        let allowed_coins = self
            .get_allowed_coins()
            .iter()
            .map(|info| info.denom.clone())
            .collect();

        self.app.instantiate_contract(
            code_id,
            self.get_owner(),
            &InstantiateMsg {
                owner: self.get_owner().to_string(),
                allowed_coins,
                allowed_vaults: self.get_allowed_vaults(),
                red_bank,
                oracle,
            },
            &[],
            "mock-rover-contract",
            None,
        )
    }

    fn get_owner(&self) -> Addr {
        self.owner
            .clone()
            .unwrap_or_else(|| Addr::unchecked("owner"))
    }

    fn get_oracle(&mut self) -> OracleBase<Addr> {
        self.oracle.clone().unwrap_or_else(|| self.setup_oracle())
    }

    fn setup_oracle(&mut self) -> OracleBase<Addr> {
        let contract_code_id = self.app.store_code(mock_oracle_contract());
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("oracle_contract_owner"),
                &OracleInstantiateMsg {
                    coins: self
                        .get_allowed_coins()
                        .iter()
                        .map(|item| CoinPrice {
                            denom: item.denom.clone(),
                            price: item.price,
                        })
                        .collect(),
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();
        OracleBase::new(addr)
    }

    fn get_red_bank(&mut self) -> RedBankBase<Addr> {
        self.red_bank
            .clone()
            .unwrap_or_else(|| self.setup_red_bank())
    }

    fn setup_red_bank(&mut self) -> RedBankBase<Addr> {
        let contract_code_id = self.app.store_code(mock_red_bank_contract());
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("red_bank_contract_owner"),
                &RedBankInstantiateMsg {
                    coins: self
                        .get_allowed_coins()
                        .iter()
                        .map(|item| CoinMarketInfo {
                            denom: item.denom.to_string(),
                            max_ltv: item.max_ltv,
                            liquidation_threshold: item.liquidation_threshold,
                        })
                        .collect(),
                },
                &[],
                "mock-red-bank",
                None,
            )
            .unwrap();

        // fund red bank with whitelisted coins
        if !self.get_allowed_coins().is_empty() {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: addr.to_string(),
                    amount: self
                        .get_allowed_coins()
                        .iter()
                        .map(|info| info.to_coin(DEFAULT_RED_BANK_COIN_BALANCE))
                        .collect(),
                }))
                .unwrap();
        }

        RedBankBase::new(addr)
    }

    fn get_allowed_vaults(&self) -> Vec<String> {
        self.allowed_vaults.clone().unwrap_or_default()
    }

    fn get_allowed_coins(&self) -> Vec<CoinInfo> {
        self.allowed_coins.clone().unwrap_or_default()
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------

    pub fn fund_account(&mut self, account: AccountToFund) -> &mut Self {
        self.accounts_to_fund.push(account);
        self
    }

    pub fn owner(&mut self, owner: &str) -> &mut Self {
        self.owner = Some(Addr::unchecked(owner));
        self
    }

    pub fn allowed_vaults(&mut self, allowed_vaults: &[String]) -> &mut Self {
        self.allowed_vaults = Some(allowed_vaults.to_vec());
        self
    }

    pub fn allowed_coins(&mut self, allowed_coins: &[CoinInfo]) -> &mut Self {
        self.allowed_coins = Some(allowed_coins.to_vec());
        self
    }

    pub fn red_bank(&mut self, red_bank: &str) -> &mut Self {
        self.red_bank = Some(RedBankBase::new(Addr::unchecked(red_bank)));
        self
    }

    pub fn oracle(&mut self, oracle: &str) -> &mut Self {
        self.oracle = Some(OracleBase::new(Addr::unchecked(oracle)));
        self
    }

    pub fn no_nft_contract(&mut self) -> &mut Self {
        self.setup_nft_contract = false;
        self
    }

    pub fn no_nft_contract_owner(&mut self) -> &mut Self {
        self.setup_nft_contract_owner = false;
        self
    }
}

//--------------------------------------------------------------------------------------------------
// Shared utils between MockBuilder & MockEnv
//--------------------------------------------------------------------------------------------------

fn setup_nft_contract(app: &mut App, owner: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    app.instantiate_contract(
        nft_contract_code_id,
        owner.clone(),
        &NftInstantiateMsg {
            name: "Rover Credit Account".to_string(),
            symbol: "RCA".to_string(),
            minter: owner.to_string(),
        },
        &[],
        "manager-mock-account-nft",
        None,
    )
    .unwrap()
}

fn propose_new_nft_contract_owner(
    app: &mut App,
    nft_contract: Addr,
    nft_contract_owner: &Addr,
    rover: &Addr,
) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: rover.to_string(),
    };
    app.execute_contract(nft_contract_owner.clone(), nft_contract, &proposal_msg, &[])
        .unwrap();
}
