use cosmwasm_std::{
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Addr, BlockInfo, Coin, ContractInfo, Env, MessageInfo, OwnedDeps, Timestamp, TransactionInfo,
};

use super::mars_mock_querier::MarsMockQuerier;

pub struct MockEnvParams {
    pub block_time: Timestamp,
    pub block_height: u64,
}

impl Default for MockEnvParams {
    fn default() -> Self {
        MockEnvParams {
            block_time: Timestamp::from_nanos(1_571_797_419_879_305_533),
            block_height: 1,
        }
    }
}

/// mock_env replacement for cosmwasm_std::testing::mock_env
pub fn mock_env(mock_env_params: MockEnvParams) -> Env {
    Env {
        block: BlockInfo {
            height: mock_env_params.block_height,
            time: mock_env_params.block_time,
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        transaction: Some(TransactionInfo {
            index: 3,
        }),
        contract: ContractInfo {
            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
        },
    }
}

pub fn mock_env_at_block_time(seconds: u64) -> Env {
    mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(seconds),
        ..Default::default()
    })
}

pub fn mock_env_at_block_height(block_height: u64) -> Env {
    mock_env(MockEnvParams {
        block_height,
        ..Default::default()
    })
}

/// quick mock info with just the sender
pub fn mock_info(sender: &str) -> MessageInfo {
    MessageInfo {
        sender: Addr::unchecked(sender),
        funds: vec![],
    }
}

/// mock_dependencies replacement for cosmwasm_std::testing::mock_dependencies
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let contract_addr = Addr::unchecked(MOCK_CONTRACT_ADDR);
    let custom_querier: MarsMockQuerier =
        MarsMockQuerier::new(MockQuerier::new(&[(contract_addr.as_ref(), contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default(),
    }
}
