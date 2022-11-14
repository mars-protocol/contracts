# Osmosis Multisig Overview

The multisig on Osmosis is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

## Set up Osmosisd

Osmosisd is the daemon for the osmosis blockchain. To install, follow this documentation: https://docs.osmosis.zone/osmosis-core/osmosisd/

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._
   
1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

   ```shell
   osmosisd query account [address] --node=[node_URL]
   ```

2. Add each public key to the keys list in your local network.

   ```shell
   osmosisd keys add [name] --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.
   ```shell
   osmosisd keys add osmosis_multisig \
   --multisig=[name1],[name2],[name3],[name4],[name5] \
   --multisig-threshold=3
   ```
4. Assert that it was completed correctly.
   ```shell
   osmosisd keys show osmosis_multisig
   ```
5. Update the config with the new mutlisig address in `outposts/scripts/deploy/osmosis/config`, which will set the owner and admin of the smart contracts to the multisig upon deployment.

## Set up environment variables 
These variables change based on the network, transaction, time, and user. Therefore, they should be provided to the multisig holders before each transaction and updated as needed on your machine.

For `# bash`:

   ```shell
   # Osmosis Testnet variables 
   export OSMO_MULTI="osmo1nxs5fw53jwh7epqnj5ypyqkdhga4lnnmng6ln5" 
   export OSMO_TEST_CHAINID="osmo-test-4" 
   export OSMO_TEST_NODE="https://rpc-test.osmosis.zone:443" 
   export OSMO_ACCOUNT="278179"
   export OSMO_TEST_ADDR_PROVIDER="osmo1h5ljap7yajt8d8kejx0xsq46evxmalwgy78xfc5arrk3g3gwgkes7l06p8"
   export OSMO_TEST_REDBANK="osmo1gtkcx8634wufu4awt42ng7srk05hpqxkfpjpvuj03f9g69qvr3ksn27j54" 
   export OSMO_TEST_INCENTIVES="osmo12caxzc4699vde8lr3ut4tsdkvsvhzruvsxlrmd4v6tamyacdymdq7l8dsy"
   export OSMO_TEST_ORACLE="osmo1eeg2uuuxk9agv8slskmhay3h5929vkfu9gfk0egwtfg9qs86w5dqty96cf"
   export OSMO_TEST_REWARDS_COLLECTOR="osmo1xl7jguvkg807ya00s0l722nwcappfzyzrac3ug5tnjassnrmnfrs47wguz"
   export OSMO_TEST_ADDR_PROVIDER_ID="3802"
   export OSMO_TEST_REDBANK_ID="3801"
   export OSMO_TEST_INCENTIVES_ID="3803"
   export OSMO_TEST_ORACLE_ID="3804"
   export OSMO_TEST_REWARDS_ID="3805"

   # Transaction specific variables (must be created at time of transaction) 
   export CODEID="new_code_ID_to_migrate_to"
   export SEQUENCE="current_account_sequence"
   export UNSIGNED="unsignedTX_filename.JSON"
   export SIGNEDTX="signedTX_filenme.JSON"
   export EXECUTE="msg_to_execute"

   # User specific variables
   export SINGLE_SIGN="your_name.JSON" 
   export OSMO_ADDR="your_wallet_address"
   ```
**Note:**

`OSMO_ACCOUNT` and `SEQUENCE` can be found by running:

```
osmosisd query account \
--node=$OSMO_TEST_NODE \
--chain-id=$OSMO_TEST_CHAINID \
$OSMO_MULTI
```

## Verifying Contracts 
1. Get the wasm binary executable on your local machine. 
   ```shell
   git clone https://github.com/mars-protocol/outposts.git
   
   git checkout <commit-id> 
   
   cd scripts 
   
   yarn compile
   ```
   Note: Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file. 
2. Download the wasm from the chain. 
   ```shell  
   osmosisd query wasm code $CODEID -- $NODE download.wasm
   ```
   
3. Verify that the diff is empty between them. If any value is returned, then the wasm files differ. 
   ```shell
   diff artifacts/$CONTRACTNAME.wasm download.wasm 
   ```
   
## Query contract configs

   * Red Bank Contract Config: 
   ``` shell
   QUERY='{"config": {}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_REDBANK "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```
   * Oracle Config:
   ``` shell
   QUERY='{"config": {}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_ORACLE "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```
   * Incentives Config:
   ``` shell
   QUERY='{"config": {}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_INCENTIVES "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```
   * Address Provider Config:
   ``` shell
   QUERY='{"config": {}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_ADDR_PROVIDER "$QUERY" --output json --node=$OSMO_TEST_NODE
  ```
  * Rewards Collector Config:
   ``` shell
   QUERY='{"config": {}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_REWARDS_COLLECTOR "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```
   * Verify OSMO and ATOM are initialized in the red bank market and have the correct params:
   ``` shell
   QUERY='{"market":{"denom":"uosmo"}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_REDBANK "$QUERY" --output json --node=$OSMO_TEST_NODE
  
   QUERY='{"market":{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_REDBANK "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```
   * Verify Oracle Price Source is set correctly:
   ``` shell
   QUERY='{"price_sources":{}}'
   osmosisd query wasm contract-state smart $OSMO_TEST_ORACLE "$QUERY" --output json --node=$OSMO_TEST_NODE
   ```

## Signing a TX with the multisig - Testnet Migrate Msg Example

**Every multisig holder is responsible for verifying the contract's newly uploaded code for every migrate msg.** 

_Note: The multisig must have at least one tx against it for the address to exist in Osmosis' state._

1. If the multisig has no txs against it, send some tokens to the account. Otherwise, the account does not exist in Osmosis' state.

2. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```shell
   osmosisd keys list
   ```

   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.

3. Ensure the newly uploaded code has a migration entry point. 
   ```rust
   #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
   pub struct MigrateMsg {}
   
   #[cfg_attr(not(feature = "library"), entry_point)]
   pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
   }
   ```
4. Initiate the multisig migrate tx. This can be done by any one of the multisig holders.

   Signing over a node:

   ```shell
   osmosisd tx wasm migrate $CONTRACT $CODEID '{}' \
   --from=$OSMO_MULTI \
   --chain-id=$OSMO_TEST_CHAINID \
   --generate-only > $UNSIGNED \
   --node=$OSMO_TEST_NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```shell
   osmosisd tx wasm migrate $CONTRACT $CODEID '{}' \
   --from=$OSMO_MULTI\
   --chain-id=$OSMO_TEST_CHAINID \
   --generate-only > $UNSIGNED \
   --offline \
   --sequence=$SEQUENCE \
   --account-number=$OSMO_ACCOUNT
   ```

5. Distribute the generated file to all signers.

6. Individually sign the transaction.
   Signing over a node:

   ```shell
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$OSMO_MULTI \
   --from=$OSMO_ADDR \
   --output-document=$SINGLE_SIGN \
   --chain-id=$OSMO_TEST_CHAINID \
   --node=$OSMO_TEST_NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```shell
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$OSMO_MULTI \
   --from=$OSMO_ADDR \
   --output-document=$SINGLE_SIGN \
   --chain-id=$OSMO_TEST_CHAINID \
   --offline \
   --sequence=$SEQUENCE \
   --account=$OSMO_ACCOUNT
   ```

7. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.
   Signing over a node:

   ```shell
   osmosisd tx multisign \
   $UNSIGNED \
   $OSMO_MULTI \
   `$SINGER1`.json `$SIGNER2`.json `$SIGNER3`.json \
   --output-document=$SIGNED \
   --chain-id=$OSMO_TEST_CHAINID \
   --node=$OSMO_TEST_NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```shell
   osmosisd tx multisign \
   $UNSIGNED \
   $OSMO_MULTI \
   `$SINGER1`.json `$SIGNER2`.json `$SIGNER3`.json \
   --output-document=$SIGNED \
   --chain-id=$OSMO_TEST_CHAINID \
   --offline \
   --sequence=$SEQUENCE \
   --account=$OSMO_ACCOUNT
   ```

8. Broadcast the transaction.
   ```shell
   osmosisd tx broadcast $SIGNED \
    --chain-id=$OSMO_TEST_CHAINID \
    --broadcast-mode=block
    --node=$OSMO_TEST_NODE
   ```
   Note: For the tx to be able to broadcast, the newly uploaded code needs to have a migration entry point, meaning you have to put an empty (returning Ok) migration method.
 
9. Verify the new contract.
   ```
   git clone https://github.com/mars-protocol/outposts.git
   
   git checkout <commit-id> 
   
   cd scripts 
   
   yarn compile
   ```
  
   ```  
   osmosisd query wasm code $CODEID $OSMO_TEST_NODE download.wasm
   ```

   ``` 
   diff artifacts/$CONTRACTNAME.wasm download.wasm 
   ```

## Signing a TX with the multisig - Testnet Execute Msg Example
Every multisig holder is responsible for verifying the execute msg inside the json file of their unsigned tx. 

1. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```
   osmosisd keys list
   ```
   
   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.
2. Initiate the multisig execute tx. This can be done by any one of the multisig holders.

   ```shell
   osmosisd tx wasm execute $CONTRACTADDR $EXECUTE \
   --from=$OSMO_MULTI \
   --chain-id=$OSMO_TEST_CHAINID \
   --generate-only > $UNSIGNED \
   --node=$OSMO_TEST_NODE
   ```
   
3. Distribute the generated file to all signers.

4. Individually sign the transaction.

   ```shell
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$OSMO_MULTI \
   --from=$OSMO_ADDR \
   --output-document=$SINGLE_SIGN \
   --chain-id=$OSMO_TEST_CHAINID \
   --node=$OSMO_TEST_NODE
   ```

5. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.

   ```shell
   osmosisd tx multisign \
   $UNSIGNED \
   $OSMO_MULTI \
   `$SINGER1`.json `$SIGNER2`.json `$SIGNER3`.json \
   --output-document=$SIGNED \
   --chain-id=$OSMO_TEST_CHAINID \
   --node=$OSMO_TEST_NODE
   ```

6. Broadcast the transaction.
   ```shell
   osmosisd tx broadcast $SIGNED \
    --chain-id=$OSMO_TEST_CHAINID \
    --broadcast-mode=block
    --node=$OSMO_TEST_NODE
   ```
   
## Examples of Execute Args:
For this to be completed as a multisig tx, the flags and steps from the previous section must be used. 
```shell
# Red Bank 
EXECUTE='{"deposit":{}}'
osmosisd tx wasm execute $REDBANKADDR "$EXECUTE" 


EXECUTE='{"update_uncollateralized_loan_limit":{"user":"$ADDR","denom":"$DENOM","new_limit":"1000000000"}}'
osmosisd tx wasm execute $REDBANKADDR "$EXECUTE" 

# Rewards Collector
EXECUTE='{"update_config":{"new_cfg": {"safety_fund_denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","fee_collector_denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"}}}'
osmosisd tx wasm execute $REWARDSADDR "$EXECUTE" 


EXECUTE='{"set_route":{"denom_in":"uosmo","denom_out":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","route":[{"token_out_denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","pool_id":"1"}]}}'
osmosisd tx wasm execute $REWARDSADDR "$EXECUTE" 


EXECUTE='{"swap_asset":{"denom":"uosmo"}}'
osmosisd tx wasm execute $REWARDSADDR "$EXECUTE" 

# Oracle 
EXECUTE='{"set_price_source":{"denom":"uosmo","price_source":{"fixed":{"price":"1.0"}}}}'
osmosisd tx wasm execute $ORACLEADDR "$EXECUTE" 


EXECUTE='{"set_price_source":{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","price_source":{"spot":{"pool_id":1}}}}'
osmosisd tx wasm execute $ORACLEADDR "$EXECUTE" 


EXECUTE='{"set_price_source":{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","price_source":{"twap":{"pool_id":1,"window_size":86400}}}}'
osmosisd tx wasm execute $ORACLEADDR "$EXECUTE" 
```