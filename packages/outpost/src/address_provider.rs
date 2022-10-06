use std::any::type_name;
use std::fmt;
use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, StdError};

/// Contracts deployed on one chain
#[cw_serde]
#[derive(Copy, Eq, Hash)]
pub enum MarsContract {
    Incentives,
    Oracle,
    RedBank,
    RewardsCollector,
}

/// Governance accounts, modules
#[cw_serde]
#[derive(Copy, Eq, Hash)]
pub enum MarsGov {
    /// Protocol admin is an ICS-27 interchain account controlled by Mars Hub's x/gov module.
    /// This account will take the owner and admin roles of outpost contracts.
    ///
    /// Owner means the account who can invoke certain priviliged execute methods on a contract,
    /// such as updating the config.
    /// Admin means the account who can migrate a contract.
    ProtocolAdmin,
    /// The `fee_collector` module account controlled by Mars Hub's x/distribution module.
    /// Funds sent to this account will be distributed as staking rewards.
    ///
    /// NOTE: This is a Mars Hub address with the `mars` bech32 prefix, which may not be recognized
    /// by the `api.addr_validate` method.
    FeeCollector,
    /// The module account controlled by the by Mars Hub's x/safety module.
    /// Funds sent to this account will be deposited into the safety fund.
    ///
    /// NOTE: This is a Mars Hub address with the `mars` bech32 prefix, which may not be recognized
    /// by the `api.addr_validate` method.
    SafetyFund,
}

impl fmt::Display for MarsContract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MarsContract::Incentives => "incentives",
            MarsContract::Oracle => "oracle",
            MarsContract::RedBank => "red_bank",
            MarsContract::RewardsCollector => "rewards_collector",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for MarsGov {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MarsGov::ProtocolAdmin => "protocol_admin",
            MarsGov::FeeCollector => "fee_collector",
            MarsGov::SafetyFund => "safety_fund",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for MarsContract {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "incentives" => Ok(MarsContract::Incentives),
            "oracle" => Ok(MarsContract::Oracle),
            "red_bank" => Ok(MarsContract::RedBank),
            "rewards_collector" => Ok(MarsContract::RewardsCollector),
            _ => Err(StdError::parse_err(type_name::<Self>(), s)),
        }
    }
}

impl FromStr for MarsGov {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fee_collector" => Ok(MarsGov::FeeCollector),
            "protocol_admin" => Ok(MarsGov::ProtocolAdmin),
            "safety_fund" => Ok(MarsGov::SafetyFund),
            _ => Err(StdError::parse_err(type_name::<Self>(), s)),
        }
    }
}

/// Essentially, mars-address-provider is a required init param for all other contracts, so it needs
/// to be initialised first (Only owner can be set on initialization). So the deployment looks like
/// this:
///
/// 1. Init the address provider
/// 2. Init all other contracts, passing in the address provider address (not ALL contracts need this
///    but many do)
/// 3. Update the address provider, with an update config call to contain all the other contract addresses
///    from step 2, this is why we need it to be owned by an EOA (externally owned account) - so we
///    can do this update as part of the deployment
/// 4. Update the owner of the address provider contract at the end of deployment to be either a. the
///    multisig or b. the gov/council contract
#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner
    pub owner: String,
    /// The address prefix of the chain this contract is deployed on
    pub prefix: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set addresses for contracts
    SetContractAddress {
        contract: MarsContract,
        address: String,
    },
    /// Set addresses for governance
    SetGovAddress {
        gov: MarsGov,
        address: String,
    },
    /// Propose to transfer the contract's ownership to another account
    TransferOwnership {
        new_owner: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(Config)]
    Config {},

    /// Get a single address for a contract
    #[returns(ContractAddressResponse)]
    ContractAddress(MarsContract),
    /// Get a list of addresses for some contracts
    #[returns(Vec<ContractAddressResponse>)]
    ContractAddresses(Vec<MarsContract>),
    /// Query all stored contracts with pagination
    #[returns(Vec<ContractAddressResponse>)]
    AllContractAddresses {
        start_after: Option<MarsContract>,
        limit: Option<u32>,
    },

    /// Get a single governance address
    #[returns(GovAddressResponse)]
    GovAddress(MarsGov),
    /// Get a list of governance addresses
    #[returns(Vec<GovAddressResponse>)]
    GovAddresses(Vec<MarsGov>),
    /// Query all stored governance addresses with pagination
    #[returns(Vec<GovAddressResponse>)]
    AllGovAddresses {
        start_after: Option<MarsGov>,
        limit: Option<u32>,
    },
}

pub type Config = InstantiateMsg;

#[cw_serde]
pub struct ContractAddressResponse {
    /// The contract
    pub contract: MarsContract,
    /// The contract's address
    pub address: Addr,
}

#[cw_serde]
pub struct GovAddressResponse {
    /// The governance account, module
    pub gov: MarsGov,
    /// The governance's address
    pub address: String,
}

pub mod helpers {
    use std::collections::HashMap;

    use super::{ContractAddressResponse, MarsContract, QueryMsg};
    use crate::address_provider::{GovAddressResponse, MarsGov};
    use cosmwasm_std::{Addr, Deps, StdResult};

    pub fn query_contract_address(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        contract: MarsContract,
    ) -> StdResult<Addr> {
        let res: ContractAddressResponse = deps
            .querier
            .query_wasm_smart(address_provider_addr, &QueryMsg::ContractAddress(contract))?;

        Ok(res.address)
    }

    pub fn query_contract_addresses(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        contracts: Vec<MarsContract>,
    ) -> StdResult<HashMap<MarsContract, Addr>> {
        let res: Vec<ContractAddressResponse> = deps
            .querier
            .query_wasm_smart(address_provider_addr, &QueryMsg::ContractAddresses(contracts))?;

        Ok(res.iter().map(|item| (item.contract, item.address.clone())).collect())
    }

    pub fn query_gov_address(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        gov: MarsGov,
    ) -> StdResult<String> {
        let res: GovAddressResponse =
            deps.querier.query_wasm_smart(address_provider_addr, &QueryMsg::GovAddress(gov))?;

        Ok(res.address)
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{MockApi, MockStorage};
    use cosmwasm_std::{
        from_binary, from_slice, to_binary, Addr, Binary, ContractResult, Empty, OwnedDeps,
        Querier, QuerierResult, QueryRequest, StdResult, SystemError, WasmQuery,
    };

    #[test]
    fn test_query_address() {
        let deps = OwnedDeps::<_, _, _, Empty> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: AddressProviderMockQuerier {},
            custom_query_type: Default::default(),
        };

        // Correctly set address is returned
        let address = helpers::query_contract_address(
            deps.as_ref(),
            &Addr::unchecked("address_provider"),
            MarsContract::RedBank,
        )
        .unwrap();

        assert_eq!(address, "red_bank".to_string());
    }

    #[test]
    fn test_query_addresses() {
        let deps = OwnedDeps::<_, _, _, Empty> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: AddressProviderMockQuerier {},
            custom_query_type: Default::default(),
        };

        // Correctly set addresses are returned
        let addresses = helpers::query_contract_addresses(
            deps.as_ref(),
            &Addr::unchecked("address_provider"),
            vec![MarsContract::Oracle, MarsContract::RedBank],
        )
        .unwrap();

        assert_eq!(addresses[&MarsContract::Oracle], "oracle".to_string());
        assert_eq!(addresses[&MarsContract::RedBank], "red_bank".to_string());
    }

    #[derive(Clone, Copy)]
    pub struct AddressProviderMockQuerier {}

    impl Querier for AddressProviderMockQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<Empty> = match from_slice(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                    .into()
                }
            };
            self.handle_query(&request)
        }
    }

    impl AddressProviderMockQuerier {
        pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            if let QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) = request
            {
                let query: StdResult<QueryMsg> = from_binary(msg);

                if let Ok(query) = query {
                    let ret: ContractResult<Binary> = match query {
                        QueryMsg::ContractAddress(contract) => {
                            let res = ContractAddressResponse {
                                contract,
                                address: Addr::unchecked(contract.to_string()),
                            };

                            to_binary(&res).into()
                        }

                        QueryMsg::ContractAddresses(contracts) => {
                            let addresses = contracts
                                .into_iter()
                                .map(|contract| ContractAddressResponse {
                                    contract,
                                    address: Addr::unchecked(contract.to_string()),
                                })
                                .collect::<Vec<_>>();

                            to_binary(&addresses).into()
                        }

                        _ => panic!("[mock]: Unsupported address provider query"),
                    };

                    return Ok(ret).into();
                }
            }

            panic!("[mock]: Unsupported wasm query");
        }
    }
}
