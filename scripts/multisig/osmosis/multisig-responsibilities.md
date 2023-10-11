# Osmosis Multisig Overview

The multisig on Osmosis is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

The Osmosis multisig being used for this project is `osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n`

## Verifying Contracts

### The multisig holders are responsible for verifying the contracts at the time of deployment and at the time of any contract migration.

1. Get the wasm binary executable on your local machine.

   For account-nft, credit-manager, swapper, and zapper contracts:

   ```bash
   git clone https://github.com/mars-protocol/rover.git
   git checkout <commit-id>
   cargo make rust-optimizer
   ```

   Note: Intel/AMD 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.

2. Download the wasm from the chain.

   ```bash
   osmosisd query wasm code <code id> --node <rpc url> download.wasm
   ```

3. Verify that the diff is empty between them. If any value is returned, then the wasm files differ.

   ```bash
   diff artifacts/<contract-name>.wasm download.wasm
   ```

## Query contract configs

### Multisig holders are responsible for verifying all configs are set accurately at the time of deployment at the time of any contract migration.

- Account NFT Contract Config:

  ```bash
  QUERY='{"config": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"minter": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

- Account Credit Manager Contract Config:

  ```bash
  QUERY='{"config": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"vaults_info": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"allowed_coins": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

- Account Swapper Contract Config:

  ```bash
  QUERY='{"route": { "denom_in": "uosmo", "denom_out": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2" }}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"route": { "denom_in": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "denom_out": "uosmo" }}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"route": { "denom_in": "uosmo", "denom_out": "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858" }}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"route": { "denom_in": "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858", "denom_out": "uosmo" }}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```

  ```bash
  QUERY='{"owner": {}}'
  osmosisd query wasm contract-state smart [contract_address] "$QUERY" --output json --node=[node_url]
  ```
