use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Council contract address
    pub council_address: Addr,
    /// Incentives contract address
    pub incentives_address: Addr,
    /// Safety fund contract address
    pub safety_fund_address: Addr,
    /// Mars token address
    pub mars_token_address: Addr,
    /// Oracle address
    pub oracle_address: Addr,
    /// Protocol admin address (admin for all the contracts)
    pub protocol_admin_address: Addr,
    /// Protocol Rewards Collector address
    pub protocol_rewards_collector_address: Addr,
    /// Red bank contract address
    pub red_bank_address: Addr,
    /// Staking contract address
    pub staking_address: Addr,
    /// Treasury contract address
    pub treasury_address: Addr,
    /// Vesting contract address
    pub vesting_address: Addr,
    /// xMars token address
    pub xmars_token_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Contracts from mars protocol
pub enum MarsContract {
    Council,
    Incentives,
    SafetyFund,
    MarsToken,
    Oracle,
    ProtocolAdmin,
    ProtocolRewardsCollector,
    RedBank,
    Staking,
    Treasury,
    Vesting,
    XMarsToken,
}

pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use super::MarsContract;

    /// Only owner can be set on initialization (the EOA doing all the deployments)
    /// as all other contracts are supposed to be initialized after this one with its address
    /// passed as a param.
    /// After initializing all contracts. An update config call should be done setting council as the
    /// owner and submiting all the contract addresses
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub owner: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Update address provider config
        UpdateConfig { config: ConfigParams },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, JsonSchema)]
    pub struct ConfigParams {
        /// Contract owner (has special permissions to update parameters)
        pub owner: Option<String>,
        /// Council contract handles the submission and execution of proposals
        pub council_address: Option<String>,
        /// Incentives contract handles incentives to depositors on the red bank
        pub incentives_address: Option<String>,
        /// Safety fund contract accumulates UST to protect the protocol from shortfall
        /// events
        pub safety_fund_address: Option<String>,
        /// Mars token cw20 contract
        pub mars_token_address: Option<String>,
        /// Oracle contract provides prices in uusd for assets used in the protocol
        pub oracle_address: Option<String>,
        /// Protocol admin is the Cosmos level contract admin that has permissions to migrate
        /// contracts
        pub protocol_admin_address: Option<String>,
        /// Protocol Rewards Collector receives and distributes protocl rewards
        pub protocol_rewards_collector_address: Option<String>,
        /// Red Bank contract handles user's depositing/borrowing and holds the protocol's
        /// liquidity
        pub red_bank_address: Option<String>,
        /// Staking address handles Mars staking and xMars minting
        pub staking_address: Option<String>,
        /// Treasury contract accumulates protocol fees that can be spent by the council through
        /// the voting of proposals
        pub treasury_address: Option<String>,
        /// Vesting contract
        pub vesting_address: Option<String>,
        /// xMars token cw20 contract
        pub xmars_token_address: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Get config
        Config {},
        /// Get a single address
        Address { contract: MarsContract },
        /// Get a list of addresses
        Addresses { contracts: Vec<MarsContract> },
    }
}

pub mod helpers {
    use super::msg::QueryMsg;
    use super::MarsContract;
    use crate::error::MarsError;
    use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, WasmQuery};

    pub fn query_address(
        querier: &QuerierWrapper,
        address_provider_address: Addr,
        contract: MarsContract,
    ) -> Result<Addr, MarsError> {
        let query: Addr = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address_provider_address.to_string(),
            msg: to_binary(&QueryMsg::Address {
                contract: contract.clone(),
            })?,
        }))?;

        if query == Addr::unchecked("") {
            Err(MarsError::EmptyAddresses {
                empty_addresses: vec![contract],
            })
        } else {
            Ok(query)
        }
    }

    pub fn query_addresses(
        querier: &QuerierWrapper,
        address_provider_address: Addr,
        contracts: Vec<MarsContract>,
    ) -> Result<Vec<Addr>, MarsError> {
        let expected_len = contracts.len();

        let query: Vec<Addr> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address_provider_address.to_string(),
            msg: to_binary(&QueryMsg::Addresses {
                contracts: contracts.clone(),
            })?,
        }))?;

        if query.len() != expected_len {
            return Err(MarsError::AddressesQueryWrongNumber {
                expected: expected_len as u32,
                actual: query.len() as u32,
            });
        }

        let empty_addresses = query
            .iter()
            .zip(contracts)
            .filter(|(address, _)| *address == &Addr::unchecked(""))
            .map(|(_, contract)| contract)
            .collect::<Vec<MarsContract>>();

        if !empty_addresses.is_empty() {
            Err(MarsError::EmptyAddresses { empty_addresses })
        } else {
            Ok(query)
        }
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_provider::msg::QueryMsg;
    use crate::error::MarsError;
    use cosmwasm_std::testing::{MockApi, MockStorage};
    use cosmwasm_std::{
        from_binary, from_slice, to_binary, Binary, ContractResult, OwnedDeps, Querier,
        QuerierResult, QueryRequest, StdResult, SystemError, WasmQuery,
    };
    use terra_cosmwasm::TerraQueryWrapper;

    #[test]
    fn test_query_address() {
        let deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: AddressProviderMockQuerier {},
        };

        // Errors if address is empty
        {
            let err = helpers::query_address(
                &deps.as_ref().querier,
                Addr::unchecked("address_provider"),
                MarsContract::Incentives,
            )
            .unwrap_err();

            assert_eq!(
                err,
                MarsError::EmptyAddresses {
                    empty_addresses: vec![MarsContract::Incentives]
                }
            );
        }

        // Correctly set address is returned
        {
            let address = helpers::query_address(
                &deps.as_ref().querier,
                Addr::unchecked("address_provider"),
                MarsContract::RedBank,
            )
            .unwrap();

            assert_eq!(address, Addr::unchecked("red_bank"));
        }
    }

    #[test]
    fn test_query_addresses() {
        let deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: AddressProviderMockQuerier {},
        };

        // Errors if addresses are empty
        {
            let err = helpers::query_addresses(
                &deps.as_ref().querier,
                Addr::unchecked("address_provider"),
                vec![
                    MarsContract::ProtocolRewardsCollector,
                    MarsContract::RedBank,
                    MarsContract::Incentives,
                ],
            )
            .unwrap_err();

            assert_eq!(
                err,
                MarsError::EmptyAddresses {
                    empty_addresses: vec![
                        MarsContract::ProtocolRewardsCollector,
                        MarsContract::Incentives
                    ]
                }
            );
        }

        // Correctly set addresses are returned
        {
            let addresses = helpers::query_addresses(
                &deps.as_ref().querier,
                Addr::unchecked("address_provider"),
                vec![MarsContract::Vesting, MarsContract::RedBank],
            )
            .unwrap();

            assert_eq!(
                addresses,
                vec![Addr::unchecked("vesting"), Addr::unchecked("red_bank")]
            );
        }
    }

    #[derive(Clone, Copy)]
    pub struct AddressProviderMockQuerier {}

    impl Querier for AddressProviderMockQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
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
        pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
            if let QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) = request
            {
                let parse_address_provider_query: StdResult<QueryMsg> = from_binary(msg);

                if let Ok(address_provider_query) = parse_address_provider_query {
                    let ret: ContractResult<Binary> = match address_provider_query {
                        QueryMsg::Address { contract } => {
                            to_binary(&get_contract_address(contract)).into()
                        }

                        QueryMsg::Addresses { contracts } => {
                            let addresses = contracts
                                .into_iter()
                                .map(get_contract_address)
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

    fn get_contract_address(contract: MarsContract) -> Addr {
        match contract {
            // empty for testing purposes
            MarsContract::Incentives => Addr::unchecked(""),
            MarsContract::ProtocolRewardsCollector => Addr::unchecked(""),

            // correctly set
            MarsContract::Council => Addr::unchecked("council"),
            MarsContract::SafetyFund => Addr::unchecked("safety_fund"),
            MarsContract::MarsToken => Addr::unchecked("mars_token"),
            MarsContract::Oracle => Addr::unchecked("oracle"),
            MarsContract::ProtocolAdmin => Addr::unchecked("protocol_admin"),
            MarsContract::RedBank => Addr::unchecked("red_bank"),
            MarsContract::Staking => Addr::unchecked("staking"),
            MarsContract::Treasury => Addr::unchecked("treasury"),
            MarsContract::Vesting => Addr::unchecked("vesting"),
            MarsContract::XMarsToken => Addr::unchecked("xmars_token"),
        }
    }
}
