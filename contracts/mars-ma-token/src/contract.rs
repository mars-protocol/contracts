use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ReceiveMsg};
use cw20_base::allowances::{
    execute_decrease_allowance, execute_increase_allowance, query_allowance,
};
use cw20_base::contract::{
    create_accounts, execute_update_marketing, execute_upload_logo, query_balance,
    query_download_logo, query_marketing_info, query_minter, query_token_info,
};
use cw20_base::enumerable::{query_all_accounts, query_all_allowances};
use cw20_base::state::{BALANCES, TOKEN_INFO};
use cw20_base::ContractError;

use mars_core::cw20_core::instantiate_token_info_and_marketing;
use mars_core::red_bank;

use crate::allowances::{execute_send_from, execute_transfer_from};
use crate::core;
use crate::msg::{BalanceAndTotalSupplyResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::Config;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ma-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let base_msg = cw20_base::msg::InstantiateMsg {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        initial_balances: msg.initial_balances,
        mint: msg.mint,
        marketing: msg.marketing,
    };
    base_msg.validate()?;

    let total_supply = create_accounts(&mut deps, &base_msg.initial_balances)?;
    instantiate_token_info_and_marketing(&mut deps, base_msg, total_supply)?;

    // store token config
    CONFIG.save(
        deps.storage,
        &Config {
            red_bank_address: deps.api.addr_validate(&msg.red_bank_address)?,
            incentives_address: deps.api.addr_validate(&msg.incentives_address)?,
        },
    )?;

    let mut res = Response::new();
    if let Some(hook) = msg.init_hook {
        res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.contract_addr,
            msg: hook.msg,
            funds: vec![],
        }));
    }

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::TransferOnLiquidation {
            sender,
            recipient,
            amount,
        } => execute_transfer_on_liquidation(deps, env, info, sender, recipient, amount),
        ExecuteMsg::Burn { user, amount } => execute_burn(deps, env, info, user, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => execute_update_marketing(deps, env, info, project, description, marketing),
        ExecuteMsg::UploadLogo(logo) => execute_upload_logo(deps, env, info, logo),
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient_unchecked: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let config = CONFIG.load(deps.storage)?;

    let recipient = deps.api.addr_validate(&recipient_unchecked)?;
    let messages = core::transfer(
        deps.storage,
        &config,
        info.sender.clone(),
        recipient,
        amount,
        true,
    )?;

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient_unchecked)
        .add_attribute("amount", amount)
        .add_messages(messages);
    Ok(res)
}

pub fn execute_transfer_on_liquidation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender_unchecked: String,
    recipient_unchecked: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // only red bank can call
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.red_bank_address {
        return Err(ContractError::Unauthorized {});
    }

    let sender = deps.api.addr_validate(&sender_unchecked)?;
    let recipient = deps.api.addr_validate(&recipient_unchecked)?;

    let messages = core::transfer(deps.storage, &config, sender, recipient, amount, false)?;

    let res = Response::new()
        .add_messages(messages)
        .add_attribute("action", "transfer")
        .add_attribute("from", sender_unchecked)
        .add_attribute("to", recipient_unchecked)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_unchecked: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // only money market can burn
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.red_bank_address {
        return Err(ContractError::Unauthorized {});
    }

    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // lower balance
    let user_address = deps.api.addr_validate(&user_unchecked)?;
    let user_balance_before = core::decrease_balance(deps.storage, &user_address, amount)?;

    // reduce total_supply
    let mut total_supply_before = Uint128::zero();
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        total_supply_before = info.total_supply;
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    let res = Response::new()
        .add_message(core::balance_change_msg(
            config.incentives_address,
            user_address,
            user_balance_before,
            total_supply_before,
        )?)
        .add_attribute("action", "burn")
        .add_attribute("user", user_unchecked)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient_unchecked: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let mut token_info = TOKEN_INFO.load(deps.storage)?;
    if token_info.mint.is_none() || token_info.mint.as_ref().unwrap().minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let total_supply_before = token_info.total_supply;

    // update supply and enforce cap
    token_info.total_supply += amount;
    if let Some(limit) = token_info.get_cap() {
        if token_info.total_supply > limit {
            return Err(ContractError::CannotExceedCap {});
        }
    }
    TOKEN_INFO.save(deps.storage, &token_info)?;

    // add amount to recipient balance
    let rcpt_address = deps.api.addr_validate(&recipient_unchecked)?;
    let rcpt_balance_before = core::increase_balance(deps.storage, &rcpt_address, amount)?;

    let config = CONFIG.load(deps.storage)?;

    let res = Response::new()
        .add_message(core::balance_change_msg(
            config.incentives_address,
            rcpt_address,
            rcpt_balance_before,
            total_supply_before,
        )?)
        .add_attribute("action", "mint")
        .add_attribute("to", recipient_unchecked)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract_unchecked: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // move the tokens to the contract
    let config = CONFIG.load(deps.storage)?;
    let contract_address = deps.api.addr_validate(&contract_unchecked)?;

    let transfer_messages = core::transfer(
        deps.storage,
        &config,
        info.sender.clone(),
        contract_address,
        amount,
        true,
    )?;

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("to", &contract_unchecked)
        .add_attribute("amount", amount)
        .add_messages(transfer_messages)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.to_string(),
                amount,
                msg,
            }
            .into_cosmos_msg(contract_unchecked)?,
        );

    Ok(res)
}

// QUERY

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::BalanceAndTotalSupply { address } => {
            to_binary(&query_balance_and_total_supply(deps, address)?)
        }
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_binary(&query_download_logo(deps)?),
        QueryMsg::UnderlyingAssetBalance { address } => {
            to_binary(&query_underlying_asset_balance(deps, env, address)?)
        }
    }
}

fn query_balance_and_total_supply(
    deps: Deps,
    address_unchecked: String,
) -> StdResult<BalanceAndTotalSupplyResponse> {
    let address = deps.api.addr_validate(&address_unchecked)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    let info = TOKEN_INFO.load(deps.storage)?;
    Ok(BalanceAndTotalSupplyResponse {
        balance,
        total_supply: info.total_supply,
    })
}

pub fn query_underlying_asset_balance(
    deps: Deps,
    env: Env,
    address: String,
) -> StdResult<BalanceResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();

    let config = CONFIG.load(deps.storage)?;

    let query: Uint128 = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.red_bank_address.into(),
        msg: to_binary(&red_bank::msg::QueryMsg::UnderlyingLiquidityAmount {
            ma_token_address: env.contract.address.into(),
            amount_scaled: balance,
        })?,
    }))?;

    Ok(BalanceResponse { balance: query })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr, CosmosMsg, StdError, SubMsg, WasmMsg};

    use cw20::{
        Cw20Coin, Logo, LogoInfo, MarketingInfoResponse, MinterResponse, TokenInfoResponse,
    };
    use cw20_base::msg::InstantiateMarketingInfo;

    use super::*;
    use crate::msg::InitHook;
    use crate::test_helpers::{do_instantiate, do_instantiate_with_minter, get_balance};

    mod instantiate {
        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies(&[]);
            let amount = Uint128::from(11223344u128);
            let hook_msg = Binary::from(r#"{"some": 123}"#.as_bytes());
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: String::from("addr0000"),
                    amount,
                }],
                mint: None,
                marketing: None,
                init_hook: Some(InitHook {
                    contract_addr: String::from("hook_dest"),
                    msg: hook_msg.clone(),
                }),
                red_bank_address: String::from("red_bank"),
                incentives_address: String::from("incentives"),
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("hook_dest"),
                    msg: hook_msg,
                    funds: vec![],
                }))]
            );

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
        }

        #[test]
        fn mintable() {
            let mut deps = mock_dependencies(&[]);
            let amount = Uint128::new(11223344);
            let minter = String::from("asmodat");
            let limit = Uint128::new(511223344);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
                init_hook: None,
                red_bank_address: String::from("red_bank"),
                incentives_address: String::from("incentives"),
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
            assert_eq!(
                query_minter(deps.as_ref()).unwrap(),
                Some(MinterResponse {
                    minter,
                    cap: Some(limit),
                }),
            );
        }

        #[test]
        fn mintable_over_cap() {
            let mut deps = mock_dependencies(&[]);
            let amount = Uint128::new(11223344);
            let minter = String::from("asmodat");
            let limit = Uint128::new(11223300);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: String::from("addr0000"),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter,
                    cap: Some(limit),
                }),
                marketing: None,
                init_hook: None,
                red_bank_address: String::from("red_bank"),
                incentives_address: String::from("incentives"),
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let err = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap_err();
            assert_eq!(
                err,
                StdError::generic_err("Initial supply greater than cap").into()
            );
        }

        mod marketing {
            use super::*;

            #[test]
            fn basic() {
                let mut deps = mock_dependencies(&[]);
                let instantiate_msg = InstantiateMsg {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    initial_balances: vec![],
                    mint: None,
                    marketing: Some(InstantiateMarketingInfo {
                        project: Some("Project".to_owned()),
                        description: Some("Description".to_owned()),
                        marketing: Some("marketing".to_owned()),
                        logo: Some(Logo::Url("url".to_owned())),
                    }),
                    init_hook: None,
                    red_bank_address: String::from("red_bank"),
                    incentives_address: String::from("incentives"),
                };

                let info = mock_info("creator", &[]);
                let env = mock_env();
                let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
                assert_eq!(0, res.messages.len());

                assert_eq!(
                    query_marketing_info(deps.as_ref()).unwrap(),
                    MarketingInfoResponse {
                        project: Some("Project".to_owned()),
                        description: Some("Description".to_owned()),
                        marketing: Some(Addr::unchecked("marketing")),
                        logo: Some(LogoInfo::Url("url".to_owned())),
                    }
                );

                let err = query_download_logo(deps.as_ref()).unwrap_err();
                assert!(
                    matches!(err, StdError::NotFound { .. }),
                    "Expected StdError::NotFound, received {}",
                    err
                );
            }

            #[test]
            fn invalid_marketing() {
                let mut deps = mock_dependencies(&[]);
                let instantiate_msg = InstantiateMsg {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    initial_balances: vec![],
                    mint: None,
                    marketing: Some(InstantiateMarketingInfo {
                        project: Some("Project".to_owned()),
                        description: Some("Description".to_owned()),
                        marketing: Some("m".to_owned()),
                        logo: Some(Logo::Url("url".to_owned())),
                    }),
                    init_hook: None,
                    red_bank_address: String::from("red_bank"),
                    incentives_address: String::from("incentives"),
                };

                let info = mock_info("creator", &[]);
                let env = mock_env();
                instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap_err();

                let err = query_download_logo(deps.as_ref()).unwrap_err();
                assert!(
                    matches!(err, StdError::NotFound { .. }),
                    "Expected StdError::NotFound, received {}",
                    err
                );
            }
        }
    }

    #[test]
    fn can_mint_by_minter() {
        let mut deps = mock_dependencies(&[]);

        let genesis = String::from("genesis");
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);
        do_instantiate_with_minter(deps.as_mut(), &genesis, amount, &minter, Some(limit));

        // minter can mint coins to some winner
        let winner = String::from("lucky");
        let prize = Uint128::new(222_222_222);
        let msg = ExecuteMsg::Mint {
            recipient: winner.clone(),
            amount: prize,
        };

        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("incentives"),
                msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                    user_address: Addr::unchecked(&winner),
                    user_balance_before: Uint128::zero(),
                    total_supply_before: amount,
                },)
                .unwrap(),
                funds: vec![],
            })),]
        );
        assert_eq!(get_balance(deps.as_ref(), genesis), amount);
        assert_eq!(get_balance(deps.as_ref(), winner.clone()), prize);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount + prize
        );

        // but cannot mint nothing
        let msg = ExecuteMsg::Mint {
            recipient: winner.clone(),
            amount: Uint128::zero(),
        };
        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // but if it exceeds cap (even over multiple rounds), it fails
        // cap is enforced
        let msg = ExecuteMsg::Mint {
            recipient: winner,
            amount: Uint128::new(333_222_222),
        };
        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::CannotExceedCap {});
    }

    #[test]
    fn others_cannot_mint() {
        let mut deps = mock_dependencies(&[]);
        do_instantiate_with_minter(
            deps.as_mut(),
            &String::from("genesis"),
            Uint128::new(1234),
            &String::from("minter"),
            None,
        );

        let msg = ExecuteMsg::Mint {
            recipient: String::from("lucky"),
            amount: Uint128::new(222),
        };
        let info = mock_info("anyone else", &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn no_one_mints_if_minter_unset() {
        let mut deps = mock_dependencies(&[]);
        do_instantiate(deps.as_mut(), &String::from("genesis"), Uint128::new(1234));

        let msg = ExecuteMsg::Mint {
            recipient: String::from("lucky"),
            amount: Uint128::new(222),
        };
        let info = mock_info("genesis", &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn instantiate_multiple_accounts() {
        let mut deps = mock_dependencies(&[]);
        let amount1 = Uint128::from(11223344u128);
        let addr1 = String::from("addr0001");
        let amount2 = Uint128::from(7890987u128);
        let addr2 = String::from("addr0002");
        let instantiate_msg = InstantiateMsg {
            name: "Bash Shell".to_string(),
            symbol: "BASH".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: addr1.clone(),
                    amount: amount1,
                },
                Cw20Coin {
                    address: addr2.clone(),
                    amount: amount2,
                },
            ],
            mint: None,
            marketing: None,
            init_hook: None,
            red_bank_address: String::from("red_bank"),
            incentives_address: String::from("incentives"),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Bash Shell".to_string(),
                symbol: "BASH".to_string(),
                decimals: 6,
                total_supply: amount1 + amount2,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr1), amount1);
        assert_eq!(get_balance(deps.as_ref(), addr2), amount2);
    }

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot transfer nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: too_much,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // cannot send from empty account
        let info = mock_info(addr2.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // cannot send to self
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Std(StdError::generic_err(
                "Sender and recipient cannot be the same"
            ))
        );

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("red_bank"),
                    msg: to_binary(&red_bank::msg::ExecuteMsg::FinalizeLiquidityTokenTransfer {
                        sender_address: Addr::unchecked(&addr1),
                        recipient_address: Addr::unchecked(&addr2),
                        sender_previous_balance: amount1,
                        recipient_previous_balance: Uint128::zero(),
                        amount: transfer,
                    })
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("incentives"),
                    msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                        user_address: Addr::unchecked(&addr1),
                        user_balance_before: amount1,
                        total_supply_before: amount1,
                    },)
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("incentives"),
                    msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                        user_address: Addr::unchecked(&addr2),
                        user_balance_before: Uint128::zero(),
                        total_supply_before: amount1,
                    },)
                    .unwrap(),
                    funds: vec![],
                })),
            ],
        );

        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn transfer_on_liquidation() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot transfer nothing
        {
            let info = mock_info("red_bank", &[]);
            let env = mock_env();
            let msg = ExecuteMsg::TransferOnLiquidation {
                sender: addr1.clone(),
                recipient: addr2.clone(),
                amount: Uint128::zero(),
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, ContractError::InvalidZeroAmount {});
        }

        // cannot send more than we have
        {
            let info = mock_info("red_bank", &[]);
            let env = mock_env();
            let msg = ExecuteMsg::TransferOnLiquidation {
                sender: addr1.clone(),
                recipient: addr2.clone(),
                amount: too_much,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));
        }

        // cannot send from empty account
        {
            let info = mock_info("red_bank", &[]);
            let env = mock_env();
            let msg = ExecuteMsg::TransferOnLiquidation {
                sender: addr2.clone(),
                recipient: addr1.clone(),
                amount: transfer,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));
        }

        // only money market can call transfer on liquidation
        {
            let info = mock_info(addr1.as_ref(), &[]);
            let env = mock_env();
            let msg = ExecuteMsg::TransferOnLiquidation {
                sender: addr1.clone(),
                recipient: addr2.clone(),
                amount: transfer,
            };
            let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(res_error, ContractError::Unauthorized {});
        }

        // valid transfer on liquidation
        {
            let info = mock_info("red_bank", &[]);
            let env = mock_env();
            let msg = ExecuteMsg::TransferOnLiquidation {
                sender: addr1.clone(),
                recipient: addr2.clone(),
                amount: transfer,
            };
            let res = execute(deps.as_mut(), env, info, msg).unwrap();
            assert_eq!(
                res.messages,
                vec![
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: String::from("incentives"),
                        msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                            user_address: Addr::unchecked(&addr1),
                            user_balance_before: amount1,
                            total_supply_before: amount1,
                        },)
                        .unwrap(),
                        funds: vec![],
                    })),
                    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: String::from("incentives"),
                        msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                            user_address: Addr::unchecked(&addr2),
                            user_balance_before: Uint128::zero(),
                            total_supply_before: amount1,
                        },)
                        .unwrap(),
                        funds: vec![],
                    })),
                ]
            );

            let remainder = amount1.checked_sub(transfer).unwrap();
            assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
            assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
            assert_eq!(
                query_token_info(deps.as_ref()).unwrap().total_supply,
                amount1
            );
        }
    }

    #[test]
    fn burn() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(12340000u128);
        let burn = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot burn nothing
        let info = mock_info("red_bank", &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            user: addr1.clone(),
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // cannot burn more than we have
        let info = mock_info("red_bank", &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            user: addr1.clone(),
            amount: too_much,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // only red bank can burn
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            user: addr1.clone(),
            amount: burn,
        };
        let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res_error, ContractError::Unauthorized {});
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // valid burn reduces total supply
        let info = mock_info("red_bank", &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            user: addr1.clone(),
            amount: burn,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("incentives"),
                msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                    user_address: Addr::unchecked(&addr1),
                    user_balance_before: amount1,
                    total_supply_before: amount1,
                },)
                .unwrap(),
                funds: vec![],
            })),]
        );

        let remainder = amount1.checked_sub(burn).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            remainder
        );
    }

    #[test]
    fn send() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = String::from("addr0001");
        let contract = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot send nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: Uint128::zero(),
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: too_much,
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: transfer,
            msg: send_msg.clone(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // ensure proper send message sent
        // this is the message we want delivered to the other side
        let binary_msg = Cw20ReceiveMsg {
            sender: addr1.clone(),
            amount: transfer,
            msg: send_msg,
        }
        .into_binary()
        .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("red_bank"),
                    msg: to_binary(
                        &mars_core::red_bank::msg::ExecuteMsg::FinalizeLiquidityTokenTransfer {
                            sender_address: Addr::unchecked(&addr1),
                            recipient_address: Addr::unchecked(&contract),
                            sender_previous_balance: amount1,
                            recipient_previous_balance: Uint128::zero(),
                            amount: transfer,
                        }
                    )
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("incentives"),

                    msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                        user_address: Addr::unchecked(&addr1),
                        user_balance_before: amount1,
                        total_supply_before: amount1,
                    },)
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("incentives"),
                    msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
                        user_address: Addr::unchecked(&contract),
                        user_balance_before: Uint128::zero(),
                        total_supply_before: amount1,
                    },)
                    .unwrap(),
                    funds: vec![],
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract.clone(),
                    msg: binary_msg,
                    funds: vec![],
                })),
            ]
        );

        // ensure balance is properly transferred
        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), contract), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }
}
