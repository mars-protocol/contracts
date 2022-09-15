use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coins, Addr, Coin, Decimal, Uint128};
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
use mock_vault::contract::DEFAULT_VAULT_TOKEN_PREFUND;
use mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use rover::adapters::swap::QueryMsg::EstimateExactInSwap;
use rover::adapters::swap::{
    EstimateExactInSwapResponse, InstantiateMsg as SwapperInstantiateMsg, Swapper, SwapperBase,
};
use rover::adapters::{OracleBase, RedBankBase, Vault, VaultBase, VaultUnchecked};
use rover::msg::execute::{Action, CallbackMsg};
use rover::msg::instantiate::ConfigUpdates;
use rover::msg::query::{
    CoinBalanceResponseItem, ConfigResponse, DebtShares, HealthResponse,
    PositionsWithValueResponse, SharesResponseItem, VaultPositionResponseItem, VaultWithBalance,
};
use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{
    mock_account_nft_contract, mock_oracle_contract, mock_red_bank_contract, mock_rover_contract,
    mock_swapper_contract, mock_vault_contract, AccountToFund, CoinInfo, VaultTestInfo,
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(1_000_000u128);

pub struct MockEnv {
    pub app: BasicApp,
    pub rover: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub owner: Option<Addr>,
    pub allowed_vaults: Option<Vec<VaultTestInfo>>,
    pub pre_deployed_vaults: Option<Vec<VaultUnchecked>>,
    pub allowed_coins: Option<Vec<CoinInfo>>,
    pub oracle: Option<OracleBase<Addr>>,
    pub red_bank: Option<RedBankBase<Addr>>,
    pub deploy_nft_contract: bool,
    pub set_nft_contract_owner: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
    pub max_liquidation_bonus: Option<Decimal>,
    pub max_close_factor: Option<Decimal>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            owner: None,
            allowed_vaults: None,
            pre_deployed_vaults: None,
            allowed_coins: None,
            oracle: None,
            red_bank: None,
            deploy_nft_contract: true,
            set_nft_contract_owner: true,
            accounts_to_fund: vec![],
            max_liquidation_bonus: None,
            max_close_factor: None,
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

    pub fn invoke_callback(
        &mut self,
        sender: &Addr,
        callback: CallbackMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(callback),
            &[],
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

    pub fn deploy_nft_contract(&mut self) -> AnyResult<Addr> {
        let nft_contract = deploy_nft_contract(&mut self.app, &self.rover.clone());
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
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    ) -> Vec<VaultUnchecked> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllowedVaults { start_after, limit },
            )
            .unwrap()
    }

    pub fn get_vault(&self, vault: &VaultTestInfo) -> VaultUnchecked {
        self.query_allowed_vaults(None, Some(30)) // Max limit
            .iter()
            .find(|v| {
                let info = v
                    .check(&MockApi::default())
                    .unwrap()
                    .query_vault_info(&self.app.wrap())
                    .unwrap();
                vault.lp_token_denom == info.token_denom
            })
            .unwrap()
            .clone()
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

    pub fn query_preview_redeem(&self, vault: &VaultUnchecked, shares: Uint128) -> Vec<Coin> {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_redeem_preview(&self.app.wrap(), shares)
            .unwrap()
    }

    pub fn query_total_vault_coin_balance(&self, vault: &VaultUnchecked) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::TotalVaultCoinBalance {
                    vault: vault.clone(),
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_positions(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<VaultPositionResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllVaultPositions { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_all_total_vault_coin_balances(
        &self,
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    ) -> Vec<VaultWithBalance> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalVaultCoinBalances { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_swap_estimate(
        &self,
        coin_in: &Coin,
        denom_out: &str,
    ) -> EstimateExactInSwapResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.swapper,
                &EstimateExactInSwap {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.to_string(),
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

        if self.deploy_nft_contract {
            let nft_contract = deploy_nft_contract(&mut self.app, &nft_contract_owner);
            if self.set_nft_contract_owner {
                propose_new_nft_contract_owner(
                    &mut self.app,
                    nft_contract.clone(),
                    &nft_contract_owner,
                    rover,
                );
                self.update_config(
                    rover,
                    ConfigUpdates {
                        account_nft: Some(nft_contract.to_string()),
                        ..Default::default()
                    },
                )
            }
        }
    }

    pub fn update_config(&mut self, rover: &Addr, new_config: ConfigUpdates) {
        self.app
            .execute_contract(
                self.get_owner(),
                rover.clone(),
                &ExecuteMsg::UpdateConfig { new_config },
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Get or defaults
    //--------------------------------------------------------------------------------------------------

    fn get_rover(&mut self) -> AnyResult<Addr> {
        let code_id = self.app.store_code(mock_rover_contract());
        let oracle = self.get_oracle().into();
        let red_bank = self.get_red_bank().into();
        let swapper = self.deploy_swapper().into();
        let allowed_coins = self
            .get_allowed_coins()
            .iter()
            .map(|info| info.denom.clone())
            .collect();
        let max_liquidation_bonus = self.get_max_liquidation_bonus();
        let max_close_factor = self.get_max_close_factor();

        let mut allowed_vaults = vec![];
        allowed_vaults.extend(self.deploy_vaults());
        allowed_vaults.extend(self.pre_deployed_vaults.clone().unwrap_or_default());

        self.app.instantiate_contract(
            code_id,
            self.get_owner(),
            &InstantiateMsg {
                owner: self.get_owner().to_string(),
                allowed_coins,
                allowed_vaults,
                red_bank,
                oracle,
                max_liquidation_bonus,
                max_close_factor,
                swapper,
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
        if self.oracle.is_none() {
            let addr = self.deploy_oracle();
            self.oracle = Some(addr);
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) -> OracleBase<Addr> {
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
        if self.red_bank.is_none() {
            let addr = self.deploy_red_bank();
            self.red_bank = Some(addr);
        }
        self.red_bank.clone().unwrap()
    }

    pub fn deploy_red_bank(&mut self) -> RedBankBase<Addr> {
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

    fn deploy_vault(&mut self, vault: &VaultTestInfo) -> Vault {
        let code_id = self.app.store_code(mock_vault_contract());
        let oracle = self.get_oracle().into();
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("vault-instantiator"),
                &VaultInstantiateMsg {
                    lp_token_denom: vault.clone().lp_token_denom,
                    lockup: vault.lockup,
                    asset_denoms: vault.clone().asset_denoms,
                    oracle,
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.fund_vault(&addr, &vault.lp_token_denom);
        VaultBase::new(addr)
    }

    fn deploy_swapper(&mut self) -> Swapper {
        let code_id = self.app.store_code(mock_swapper_contract());
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("swapper-instantiator"),
                &SwapperInstantiateMsg {
                    owner: self.get_owner().to_string(),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        // Fund with osmo to simulate swaps
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins(1_000_000, "uosmo"),
            }))
            .unwrap();
        SwapperBase::new(addr)
    }

    /// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
    /// this contract expects to be pre-funded with vault tokens and it will simulate the mint.
    fn fund_vault(&mut self, vault_addr: &Addr, denom: &str) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: vault_addr.to_string(),
                amount: vec![Coin {
                    denom: denom.into(),
                    amount: DEFAULT_VAULT_TOKEN_PREFUND,
                }],
            }))
            .unwrap();
    }

    fn deploy_vaults(&mut self) -> Vec<VaultUnchecked> {
        self.allowed_vaults
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|v| self.deploy_vault(v).into())
            .collect()
    }

    fn get_allowed_coins(&self) -> Vec<CoinInfo> {
        self.allowed_coins.clone().unwrap_or_default()
    }

    fn get_max_liquidation_bonus(&self) -> Decimal {
        self.max_liquidation_bonus
            .unwrap_or_else(|| Decimal::from_atomics(5u128, 2).unwrap()) // 5%
    }

    fn get_max_close_factor(&self) -> Decimal {
        self.max_close_factor
            .unwrap_or_else(|| Decimal::from_atomics(5u128, 1).unwrap()) // 50%
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

    pub fn allowed_vaults(&mut self, allowed_vaults: &[VaultTestInfo]) -> &mut Self {
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
        self.deploy_nft_contract = false;
        self
    }

    pub fn no_nft_contract_owner(&mut self) -> &mut Self {
        self.set_nft_contract_owner = false;
        self
    }

    pub fn pre_deployed_vaults(&mut self, vaults: &[&str]) -> &mut Self {
        let vaults = vaults
            .iter()
            .map(|v| VaultBase::new(v.to_string()))
            .collect::<Vec<_>>();
        self.pre_deployed_vaults = Some(vaults);
        self
    }

    pub fn max_liquidation_bonus(&mut self, bonus: Decimal) -> &mut Self {
        self.max_liquidation_bonus = Some(bonus);
        self
    }

    pub fn max_close_factor(&mut self, cf: Decimal) -> &mut Self {
        self.max_close_factor = Some(cf);
        self
    }
}

//--------------------------------------------------------------------------------------------------
// Shared utils between MockBuilder & MockEnv
//--------------------------------------------------------------------------------------------------

fn deploy_nft_contract(app: &mut App, owner: &Addr) -> Addr {
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
