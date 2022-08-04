use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin};
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mock_red_bank::msg::{DenomWithLTV, InstantiateMsg as RedBankInstantiateMsg};
use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::execute::ExecuteMsg;
use rover::msg::instantiate::ConfigUpdates;
use rover::msg::InstantiateMsg;

use crate::helpers::contracts::{mock_account_nft_contract, mock_contract, mock_red_bank_contract};
use crate::helpers::types::MockEnv;
use crate::helpers::{mock_oracle_contract, CoinPriceLTV};

pub fn mock_create_credit_account(
    app: &mut App,
    manager_contract_addr: &Addr,
    user: &Addr,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        user.clone(),
        manager_contract_addr.clone(),
        &ExecuteMsg::CreateCreditAccount {},
        &[],
    )
}

pub fn transfer_nft_contract_ownership(
    app: &mut App,
    owner: &Addr,
    nft_contract_addr: &Addr,
    manager_contract_addr: &Addr,
) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: manager_contract_addr.to_string(),
    };
    app.execute_contract(owner.clone(), nft_contract_addr.clone(), &proposal_msg, &[])
        .unwrap();

    app.execute_contract(
        owner.clone(),
        manager_contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: ConfigUpdates {
                account_nft: Some(nft_contract_addr.to_string()),
                ..Default::default()
            },
        },
        &[],
    )
    .unwrap();
}

pub fn setup_nft_contract(app: &mut App, owner: &Addr, manager_contract_addr: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let nft_contract_addr = app
        .instantiate_contract(
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
        .unwrap();

    transfer_nft_contract_ownership(app, owner, &nft_contract_addr, &manager_contract_addr);
    nft_contract_addr
}

pub fn setup_oracle(app: &mut App, coins: &Vec<CoinPriceLTV>) -> Addr {
    let contract_code_id = app.store_code(mock_oracle_contract());
    app.instantiate_contract(
        contract_code_id,
        Addr::unchecked("oracle_contract_owner"),
        &OracleInstantiateMsg {
            coins: coins
                .iter()
                .map(|item| CoinPrice {
                    denom: item.denom.to_string(),
                    price: item.price,
                })
                .collect(),
        },
        &[],
        "mock-oracle",
        None,
    )
    .unwrap()
}

pub fn setup_red_bank(app: &mut App, coins: &Vec<CoinPriceLTV>) -> Addr {
    let contract_code_id = app.store_code(mock_red_bank_contract());
    app.instantiate_contract(
        contract_code_id,
        Addr::unchecked("red_bank_contract_owner"),
        &RedBankInstantiateMsg {
            coins: coins
                .iter()
                .map(|item| DenomWithLTV {
                    denom: item.denom.to_string(),
                    max_ltv: item.max_ltv,
                })
                .collect(),
        },
        &[],
        "mock-red-bank",
        None,
    )
    .unwrap()
}

pub fn fund_red_bank(app: &mut BasicApp, red_bank_addr: String, funds: Vec<Coin>) {
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: red_bank_addr,
        amount: funds,
    }))
    .unwrap();
}

pub fn setup_credit_manager(
    mut app: &mut App,
    owner: &Addr,
    allowed_coins: Vec<CoinPriceLTV>,
    allowed_vaults: Vec<String>,
) -> MockEnv {
    let credit_manager_code_id = app.store_code(mock_contract());
    let red_bank = setup_red_bank(app, &allowed_coins);
    let oracle = setup_oracle(app, &allowed_coins);
    let manager_initiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_coins: allowed_coins
            .iter()
            .map(|item| item.denom.clone())
            .collect(),
        allowed_vaults,
        red_bank: RedBankBase(red_bank.to_string()),
        oracle: OracleBase(oracle.to_string()),
    };

    let credit_manager = app
        .instantiate_contract(
            credit_manager_code_id,
            owner.clone(),
            &manager_initiate_msg,
            &[],
            "manager-mock",
            None,
        )
        .unwrap();

    let nft = setup_nft_contract(&mut app, &owner, &credit_manager);
    MockEnv {
        credit_manager,
        oracle,
        red_bank,
        nft,
    }
}
