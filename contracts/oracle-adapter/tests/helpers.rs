use anyhow::Result as AnyResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal};
use cw_multi_test::{AppResponse, BankSudo, BasicApp, ContractWrapper, Executor, SudoMsg};
use cw_utils::Duration;
use mars_mock_oracle::contract::{
    execute as oracleExecute, instantiate as oracleInstantiate, query as oracleQuery,
};
use mars_mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mars_mock_vault::contract::{
    execute as vaultExecute, instantiate as vaultInstantiate, query as vaultQuery,
    DEFAULT_VAULT_TOKEN_PREFUND,
};
use mars_mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use mars_oracle_adapter::contract::{execute, instantiate, query};
use mars_oracle_adapter::error::ContractError;
use mars_oracle_adapter::msg::{InstantiateMsg, PricingMethod, VaultPricingInfo};
use mars_rover::adapters::vault::VaultBase;
use mars_rover::adapters::{OracleBase, OracleUnchecked};

pub fn mock_vault_info() -> VaultTestInfo {
    VaultTestInfo {
        vault_coin_denom: "yOSMO_ATOM".to_string(),
        lockup: None,
        req_denom: "GAMM_LP_12352".to_string(),
        pricing_method: PricingMethod::PreviewRedeem,
    }
}

pub fn instantiate_oracle_adapter(app: &mut BasicApp) -> Addr {
    let contract = Box::new(ContractWrapper::new(execute, instantiate, query));
    let code_id = app.store_code(contract);

    let oracle = deploy_oracle(app);
    let vault_pricing_info = deploy_vault(app, oracle.clone().into(), mock_vault_info());
    starting_vault_deposit(app, &vault_pricing_info);

    let owner = Addr::unchecked("owner");
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &InstantiateMsg {
            oracle: oracle.into(),
            vault_pricing: vec![vault_pricing_info],
            admin: owner.to_string(),
        },
        &[],
        "mars-oracle-adapter",
        None,
    )
    .unwrap()
}

#[cw_serde]
pub struct VaultTestInfo {
    pub vault_coin_denom: String,
    pub lockup: Option<Duration>,
    pub req_denom: String,
    pub pricing_method: PricingMethod,
}

fn deploy_vault(
    app: &mut BasicApp,
    oracle: OracleUnchecked,
    vault: VaultTestInfo,
) -> VaultPricingInfo {
    let contract = ContractWrapper::new(vaultExecute, vaultInstantiate, vaultQuery);
    let code_id = app.store_code(Box::new(contract));

    let addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("vault-instantiator"),
            &VaultInstantiateMsg {
                vault_token_denom: vault.clone().vault_coin_denom,
                lockup: vault.lockup,
                base_token_denom: vault.clone().req_denom,
                oracle,
            },
            &[],
            "mock-vault",
            None,
        )
        .unwrap();

    let vault_pricing_info = VaultPricingInfo {
        vault_coin_denom: vault.vault_coin_denom,
        addr,
        method: vault.pricing_method,
        base_denom: vault.req_denom,
    };
    fund_vault(app, &vault_pricing_info);
    vault_pricing_info
}

/// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
/// this contract expects to be pre-funded with vault tokens and it will simulate the mint.
fn fund_vault(app: &mut BasicApp, vault: &VaultPricingInfo) {
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: vault.addr.to_string(),
        amount: vec![Coin {
            denom: vault.vault_coin_denom.clone(),
            amount: DEFAULT_VAULT_TOKEN_PREFUND,
        }],
    }))
    .unwrap();
}

fn starting_vault_deposit(app: &mut BasicApp, vault_info: &VaultPricingInfo) {
    let user = Addr::unchecked("some_user_xyz");
    let coin_to_deposit = coin(120_042, "GAMM_LP_12352");
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: user.to_string(),
        amount: vec![coin_to_deposit.clone()],
    }))
    .unwrap();

    let vault = VaultBase::new(vault_info.addr.clone());
    let deposit_msg = vault.deposit_msg(&coin_to_deposit).unwrap();
    app.execute(user, deposit_msg).unwrap();
}

fn deploy_oracle(app: &mut BasicApp) -> OracleBase<Addr> {
    let contract = ContractWrapper::new(oracleExecute, oracleInstantiate, oracleQuery);
    let code_id = app.store_code(Box::new(contract));

    let addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("oracle_contract_owner"),
            &OracleInstantiateMsg {
                prices: vec![
                    CoinPrice {
                        denom: "uosmo".to_string(),
                        price: Decimal::from_atomics(25u128, 2).unwrap(),
                    },
                    CoinPrice {
                        denom: "uatom".to_string(),
                        price: Decimal::from_atomics(10u128, 1).unwrap(),
                    },
                    CoinPrice {
                        denom: "GAMM_LP_12352".to_string(),
                        price: Decimal::from_atomics(8745u128, 2).unwrap(),
                    },
                ],
            },
            &[],
            "mock-oracle",
            None,
        )
        .unwrap();
    OracleBase::new(addr)
}

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}
