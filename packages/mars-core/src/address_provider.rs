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
    use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};

    pub fn query_address(
        querier: &QuerierWrapper,
        address_provider_address: Addr,
        contract: MarsContract,
    ) -> StdResult<Addr> {
        let query: Addr = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address_provider_address.to_string(),
            msg: to_binary(&QueryMsg::Address { contract })?,
        }))?;

        Ok(query)
    }

    pub fn query_addresses(
        querier: &QuerierWrapper,
        address_provider_address: Addr,
        contracts: Vec<MarsContract>,
    ) -> Result<Vec<Addr>, MarsError> {
        let expected_len = contracts.len();

        let query: Vec<Addr> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address_provider_address.to_string(),
            msg: to_binary(&QueryMsg::Addresses { contracts })?,
        }))?;

        if query.len() != expected_len {
            return Err(MarsError::AddressesQueryWrongNumber {
                expected: expected_len as u32,
                actual: query.len() as u32,
            });
        }

        Ok(query)
    }
}
