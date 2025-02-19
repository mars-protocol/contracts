#![allow(dead_code)]
use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Int128, Timestamp, Uint128};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use cw_paginate::PaginationResponse;
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_testing::integration::mock_contracts::mock_rewards_collector_osmosis_contract;
use mars_types::{
    address_provider::{self, MarsAddressType},
    incentives,
    oracle::{self, ActionKind},
    params::{
        self, EmergencyUpdate,
        ExecuteMsg::{self, UpdatePerpParams},
        PerpParams, PerpParamsUpdate,
    },
    perps::{
        self, AccountingResponse, Config, ConfigUpdates, MarketResponse, MarketStateResponse,
        PnlAmounts, PositionFeesResponse, PositionResponse, PositionsByAccountResponse, TradingFee,
        VaultPositionResponse, VaultResponse,
    },
    rewards_collector::{self, RewardConfig, TransferType},
};

use super::{
    contracts::{mock_oracle_contract, mock_perps_contract},
    mock_address_provider_contract, mock_credit_manager_contract, mock_incentives_contract,
    mock_params_contract,
};

pub const ONE_HOUR_SEC: u64 = 3600u64;

pub struct MockEnv {
    app: BasicApp,
    pub owner: Addr,
    pub perps: Addr,
    pub oracle: Addr,
    pub params: Addr,
    pub credit_manager: Addr,
    pub address_provider: Addr,
    pub rewards_collector: Addr,
}

pub struct MockEnvBuilder {
    app: BasicApp,
    deployer: Addr,
    oracle_base_denom: String,
    perps_base_denom: String,
    cooldown_period: u64,
    max_positions: u8,
    protocol_fee_rate: Decimal,
    pub address_provider: Option<Addr>,
    target_vault_collateralization_ratio: Decimal,
    pub emergency_owner: Option<String>,
    deleverage_enabled: bool,
    withdraw_enabled: bool,
    max_unlocks: u8,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("deployer"),
            oracle_base_denom: "uusd".to_string(),
            perps_base_denom: "uusdc".to_string(),
            cooldown_period: 3600,
            max_positions: 4,
            protocol_fee_rate: Decimal::percent(0),
            address_provider: None,
            target_vault_collateralization_ratio: Decimal::percent(125),
            emergency_owner: None,
            deleverage_enabled: true,
            withdraw_enabled: true,
            max_unlocks: 5,
        }
    }

    pub fn fund_accounts(&mut self, addrs: &[&Addr], amount: u128, denoms: &[&str]) {
        for addr in addrs {
            let coins: Vec<_> = denoms.iter().map(|&d| coin(amount, d)).collect();
            self.fund_account(addr, &coins);
        }
    }

    pub fn fund_account(&mut self, addr: &Addr, coins: &[Coin]) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins.to_vec(),
            }))
            .unwrap();
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
    }

    pub fn increment_by_blocks(&mut self, num_of_blocks: u64) {
        self.app.update_block(|block| {
            block.height += num_of_blocks;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(num_of_blocks * 6);
        })
    }

    pub fn increment_by_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.height += seconds / 6;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(seconds);
        })
    }

    pub fn set_block_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.time = Timestamp::from_seconds(seconds);
        })
    }

    pub fn query_block_time(&self) -> u64 {
        self.app.block_info().time.seconds()
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        updates: ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::UpdateConfig {
                updates,
            },
            &[],
        )
    }

    pub fn deposit_to_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
        max_shares_receivable: Option<Uint128>,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Deposit {
                account_id: account_id.map(|s| s.to_string()),
                max_shares_receivable,
            },
            funds,
        )
    }

    pub fn unlock_from_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
        shares: Uint128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Unlock {
                account_id: account_id.map(|s| s.to_string()),
                shares,
            },
            &[],
        )
    }

    pub fn withdraw_from_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
        min_receive: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Withdraw {
                account_id: account_id.map(|s| s.to_string()),
                min_receive,
            },
            &[],
        )
    }

    pub fn execute_perp_order(
        &mut self,
        sender: &Addr,
        account_id: &str,
        denom: &str,
        size: Int128,
        reduce_only: Option<bool>,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::ExecuteOrder {
                account_id: account_id.to_string(),
                denom: denom.to_string(),
                size,
                reduce_only,
            },
            funds,
        )
    }

    pub fn close_all_positions(
        &mut self,
        sender: &Addr,
        account_id: &str,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::CloseAllPositions {
                account_id: account_id.to_string(),
                action: None,
            },
            funds,
        )
    }

    pub fn set_price(
        &mut self,
        sender: &Addr,
        denom: &str,
        price: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.oracle.clone(),
            &oracle::ExecuteMsg::<OsmosisPriceSourceUnchecked>::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSourceUnchecked::Fixed {
                    price,
                },
            },
            &[],
        )
    }

    pub fn update_perp_params(&mut self, sender: &Addr, update: PerpParamsUpdate) {
        self.app
            .execute_contract(sender.clone(), self.params.clone(), &UpdatePerpParams(update), &[])
            .unwrap();
    }

    pub fn update_market(&mut self, sender: &Addr, params: PerpParams) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::UpdateMarket {
                params,
            },
            &[],
        )
    }

    pub fn emergency_params_update(
        &mut self,
        sender: &Addr,
        update: EmergencyUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params.clone(),
            &mars_types::params::ExecuteMsg::EmergencyUpdate(update),
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_owner(&self) -> Addr {
        let res = self.query_ownership();
        Addr::unchecked(res.owner.unwrap())
    }

    pub fn query_ownership(&self) -> OwnerResponse {
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::Owner {}).unwrap()
    }

    pub fn query_config(&self) -> Config<Addr> {
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::Config {}).unwrap()
    }

    pub fn query_vault(&self) -> VaultResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Vault {
                    action: None,
                },
            )
            .unwrap()
    }

    pub fn query_market_state(&self, denom: &str) -> MarketStateResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::MarketState {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_market(&self, denom: &str) -> MarketResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Market {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_markets(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<MarketResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Markets {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_cm_vault_position(&self, account_id: &str) -> Option<VaultPositionResponse> {
        self.query_vault_position(self.credit_manager.as_str(), Some(account_id))
    }

    pub fn query_vault_position(
        &self,
        user_address: &str,
        account_id: Option<&str>,
    ) -> Option<VaultPositionResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::VaultPosition {
                    user_address: user_address.to_string(),
                    account_id: account_id.map(|s| s.to_string()),
                },
            )
            .unwrap()
    }

    pub fn query_position(&self, account_id: &str, denom: &str) -> PositionResponse {
        self.query_position_with_order_size(account_id, denom, None)
    }

    pub fn query_position_with_order_size(
        &self,
        account_id: &str,
        denom: &str,
        order_size: Option<Int128>,
    ) -> PositionResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Position {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                    order_size,
                    reduce_only: None,
                },
            )
            .unwrap()
    }

    pub fn query_positions(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<PositionResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Positions {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_positions_by_account_id(
        &self,
        account_id: &str,
        action: ActionKind,
    ) -> PositionsByAccountResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PositionsByAccount {
                    account_id: account_id.to_string(),
                    action: Some(action),
                },
            )
            .unwrap()
    }

    pub fn query_market_accounting(&self, denom: &str) -> AccountingResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::MarketAccounting {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_total_accounting(&self) -> AccountingResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.perps.clone(), &perps::QueryMsg::TotalAccounting {})
            .unwrap()
    }

    pub fn query_realized_pnl_by_account_and_market(
        &self,
        account_id: &str,
        denom: &str,
    ) -> PnlAmounts {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::RealizedPnlByAccountAndMarket {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_opening_fee(&self, denom: &str, size: Int128) -> TradingFee {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::OpeningFee {
                    denom: denom.to_string(),
                    size,
                },
            )
            .unwrap()
    }

    pub fn query_position_fees(
        &self,
        account_id: &str,
        denom: &str,
        new_size: Int128,
    ) -> PositionFeesResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PositionFees {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                    new_size,
                },
            )
            .unwrap()
    }

    pub fn query_perp_params(&self, denom: &str) -> PerpParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.clone(),
                &params::QueryMsg::PerpParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let address_provider_contract = self.get_address_provider();
        let oracle_contract = self.deploy_oracle();
        let params_contract = self.deploy_params(address_provider_contract.as_str());
        let credit_manager_contract = self.deploy_credit_manager();
        let rewards_collector_contract =
            self.deploy_rewards_collector(address_provider_contract.as_str());
        let perps_contract = self.deploy_perps(address_provider_contract.as_str());
        let incentives_contract = self.deploy_incentives(&address_provider_contract);

        self.update_address_provider(
            &address_provider_contract,
            MarsAddressType::Incentives,
            &incentives_contract,
        );
        self.update_address_provider(
            &address_provider_contract,
            MarsAddressType::Perps,
            &perps_contract,
        );

        if self.emergency_owner.is_some() {
            self.set_emergency_owner(&params_contract, &self.emergency_owner.clone().unwrap());
        }

        Ok(MockEnv {
            app: take(&mut self.app),
            owner: self.deployer.clone(),
            perps: perps_contract,
            oracle: oracle_contract,
            params: params_contract,
            credit_manager: credit_manager_contract,
            address_provider: address_provider_contract,
            rewards_collector: rewards_collector_contract,
        })
    }

    fn deploy_address_provider(&mut self) -> Addr {
        let contract = mock_address_provider_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &address_provider::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    prefix: "".to_string(),
                },
                &[],
                "mock-address-provider",
                None,
            )
            .unwrap()
    }

    fn deploy_oracle(&mut self) -> Addr {
        let contract = mock_oracle_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &oracle::InstantiateMsg::<Empty> {
                    owner: self.deployer.clone().to_string(),
                    base_denom: self.oracle_base_denom.clone(),
                    custom_init: None,
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Oracle, addr.clone());

        addr
    }

    fn deploy_params(&mut self, address_provider: &str) -> Addr {
        let contract = mock_params_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &params::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    risk_manager: None,
                    address_provider: address_provider.to_string(),
                    max_perp_params: 40,
                },
                &[],
                "mock-params",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Params, addr.clone());

        addr
    }

    fn deploy_incentives(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_incentives_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &incentives::InstantiateMsg {
                    owner: self.deployer.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    epoch_duration: 604800,
                    max_whitelisted_denoms: 10,
                },
                &[],
                "incentives",
                None,
            )
            .unwrap()
    }

    fn deploy_perps(&mut self, address_provider: &str) -> Addr {
        let code_id = self.app.store_code(mock_perps_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &perps::InstantiateMsg {
                    address_provider: address_provider.to_string(),
                    base_denom: self.perps_base_denom.clone(),
                    cooldown_period: self.cooldown_period,
                    max_positions: self.max_positions,
                    protocol_fee_rate: self.protocol_fee_rate,
                    target_vault_collateralization_ratio: self.target_vault_collateralization_ratio,
                    deleverage_enabled: self.deleverage_enabled,
                    vault_withdraw_enabled: self.withdraw_enabled,
                    max_unlocks: self.max_unlocks,
                },
                &[],
                "mock-perps",
                None,
            )
            .unwrap()
    }

    fn deploy_credit_manager(&mut self) -> Addr {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &Empty {},
                &[],
                "mock-credit-manager",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::CreditManager, addr.clone());

        addr
    }

    fn deploy_rewards_collector(&mut self, address_provider: &str) -> Addr {
        let contract = mock_rewards_collector_osmosis_contract();
        let code_id = self.app.store_code(contract);

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &rewards_collector::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    address_provider: address_provider.to_string(),
                    safety_tax_rate: Default::default(),
                    revenue_share_tax_rate: Default::default(),
                    safety_fund_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    revenue_share_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    fee_collector_config: RewardConfig {
                        target_denom: "umars".to_string(),
                        transfer_type: TransferType::Ibc,
                    },
                    channel_id: "".to_string(),
                    timeout_seconds: 1,
                    slippage_tolerance: Default::default(),
                },
                &[],
                "mock-rewards-collector",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::RewardsCollector, addr.clone());

        addr
    }

    fn set_address(&mut self, address_type: MarsAddressType, address: Addr) {
        let address_provider_addr = self.get_address_provider();

        self.app
            .execute_contract(
                self.deployer.clone(),
                address_provider_addr,
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: address.into(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_address_provider(&mut self) -> Addr {
        if self.address_provider.is_none() {
            let addr = self.deploy_address_provider();

            self.address_provider = Some(addr);
        }
        self.address_provider.clone().unwrap()
    }

    fn update_address_provider(
        &mut self,
        address_provider_addr: &Addr,
        address_type: MarsAddressType,
        addr: &Addr,
    ) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                address_provider_addr.clone(),
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: addr.to_string(),
                },
                &[],
            )
            .unwrap();
    }

    fn set_emergency_owner(&mut self, params_contract: &Addr, eo: &str) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                params_contract.clone(),
                &ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
                    emergency_owner: eo.to_string(),
                }),
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------

    pub fn oracle_base_denom(&mut self, denom: &str) -> &mut Self {
        self.oracle_base_denom = denom.to_string();
        self
    }

    pub fn perps_base_denom(&mut self, denom: &str) -> &mut Self {
        self.perps_base_denom = denom.to_string();
        self
    }

    pub fn cooldown_period(&mut self, cp: u64) -> &mut Self {
        self.cooldown_period = cp;
        self
    }

    pub fn max_positions(&mut self, max_positions: u8) -> &mut Self {
        self.max_positions = max_positions;
        self
    }

    pub fn protocol_fee_rate(&mut self, rate: Decimal) -> &mut Self {
        self.protocol_fee_rate = rate;
        self
    }

    pub fn target_vault_collaterization_ratio(&mut self, ratio: Decimal) -> &mut Self {
        self.target_vault_collateralization_ratio = ratio;
        self
    }

    pub fn withdraw_enabled(&mut self, enabled: bool) -> &mut Self {
        self.withdraw_enabled = enabled;
        self
    }

    pub fn emergency_owner(&mut self, eo: &str) -> &mut Self {
        self.emergency_owner = Some(eo.to_string());
        self
    }

    pub fn max_unlocks(&mut self, max_unlocks: u8) -> &mut Self {
        self.max_unlocks = max_unlocks;
        self
    }
}
