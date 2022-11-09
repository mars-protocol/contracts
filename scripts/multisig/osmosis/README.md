# Osmosis Multisig Overview

The multisig on Osmosis is set to have 5 multisig holders with a threshold of 3, meaning that 3 signitures are needed for any transaction to pass.

## Set up Osmosisd

Osmosisd is the daemon for the osmosis blockchain. To install, follow this documentation: https://docs.osmosis.zone/osmosis-core/osmosisd/

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._
   
1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

   ```
   osmosisd query account [address] --node=[node_URL]
   ```

2. Add each public key to the keys list in your local network.

   ```
   osmosisd keys add [name] --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.
   ```
   osmosisd keys add osmosis_multisig \
   --multisig=[name1],[name2],[name3],[name4],[name5] \
   --multisig-threshold=3
   ```
4. Assert that it was completed correctly.
   ```
   osmosisd keys show osmosis_multisig
   ```
5. Update the config with the new mutlisig address in `outposts/scripts/deploy/osmosis/config`, which will set the owner and admin of the smart contracts to the multisig upon deployment.

## Set up environment variables 
These variables change based on the network, transaction, time, and user. Therefore, they should be provided to the multisig holders before each transaction and updated as needed on your machine.

For `# bash`:

   ```shell
   # Network specific variables
   export MULTI="multisig_address"
   export CHAINID="chain_id_of_network"
   export CONTRACT="contract_address_to_migrate"
   export NODE="node_URL"
   export ACCOUNT="account_number"

   # Transaction specific variables
   export CODEID="new_code_ID_to_migrate_to"
   export SEQUENCE="current_account_sequence"
   export UNSIGNED="unsignedTX_filename.JSON"
   export SIGNEDTX="signedTX_filenme.JSON"
   export CONTRACTNAME="contract_name_from_cargo.TOML"
   export CONTRACTADDR="contract_addr_bech32"
   export ARGS="json_encoded_send_args"

   # User specific variables
   export NAME="your_name"
   export SIGNER1="signer1"
   export SIGNER2="signer2"
   export SIGNER3="signer3"
   export SIGNER4="signer4"
   export SIGNER5="signer5"
   export ADDR="your_wallet_address"
   ```

For `# zsh`:

   ```shell
   # Network specific variables
   export MULTI=(multisig_address)
   export CHAINID=(chain_id_of_network)
   export CONTRACT=(contract_address_to_migrate)
   export NODE=(node_URL)
   export ACCOUNT=(account_number)

   # Transaction specifc variables
   export CODEID=(new_code_ID_to_migrate_to)
   export SEQUENCE=(current_account_sequence)
   export UNSIGNED=(unsignedTX_filename.JSON)
   export SIGNED=(signedTX_filenme.JSON)
   export CONTRACTNAME=(contract_name_from_cargo.TOML)
   export CONTRACTADDR=(contract_addr_bech32)
   export ARGS=(json_encoded_send_args)

   # User specifc variables
   export NAME=(your_name)
   export SIGNER1=(signer1)
   export SIGNER2=(signer2)
   export SIGNER3=(signer3)
   export SIGNER4=(signer4)
   export SIGNER5=(signer5)
   export ADDR=(your_wallet_address)
   ```

## Verifying Contracts 
1. Get the wasm binary executable on your local machine. 
   ```
   git clone https://github.com/mars-protocol/outposts.git
   
   git checkout <commit-id> 
   
   cd scripts 
   
   yarn compile
   ```
   If on mac, use `yarn compile-mac` instead of `yarn compile` 

   Note: The mac compatible version of the workspace-optimizer used to compile the contracts has only been upgraded to v0.12.8, which is not compatible with the Mars Protocol Outposts contracts. Until this version has been updated to v0.12.9, contracts cannot be compiled on a Mac. 
2. Download the wasm from the chain. 
   ```  
   osmosisd query wasm code $CODEID $NODE download.wasm
   ```
   
3. Verify that the diff is empty between them. 
   ``` 
   diff artifacts/$CONTRACTNAME.wasm download.wasm 
   ```
   
## Query contract configs 

``` shell
# oracle

QUERY='{"price_sources":{}}'
osmosisd query wasm contract-state smart $ORACLEADDR "$QUERY" --output json | jq .


QUERY='{"price":{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"}}'
osmosisd query wasm contract-state smart $ORACLEADDR "$QUERY" --output json | jq .

# rewards-collector

QUERY='{"route":{"denom_in":"uosmo","denom_out":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"}}'
osmosisd query wasm contract-state smart $REWARDSADDR "$QUERY" --output json | jq .


QUERY='{"routes":{}}'
osmosisd query wasm contract-state smart $REWARDSADDR "$QUERY" --output json | jq .


QUERY='{"config":{}}'
osmosisd query wasm contract-state smart $REWARDSADDR "$QUERY" --output json | jq .

# red-bank

QUERY='{"market":{"denom":"uosmo"}}'
osmosisd query wasm contract-state smart $REDBANKADDR "$QUERY" --output json | jq .


QUERY='{"underlying_liquidity_amount":{"denom":"uosmo","amount_scaled":"$AMOUNT"}}'
osmosisd query wasm contract-state smart $REDBANKADDR "$QUERY" --output json | jq .


QUERY='{"uncollateralized_loan_limits":{"user":"$ADDR"}}'
osmosisd query wasm contract-state smart $REDBANKADDR "$QUERY" --output json | jq .
```

## Signing a TX with the multisig - Migrate Msg Example

_Note: The multisig must have at least one tx against it for the address to exist in Osmosis' state._

1. If the multisig has no txs against it, send some tokens to the account. Otherwise, the account does not exist in Osmosis' state.

2. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```
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

   ```
   osmosisd tx wasm migrate $CONTRACT $CODEID '{}' \
   --from= $MULTI \
   --chain-id= $CHAINID \
   --generate-only > $UNSIGNED \
   --node=$NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```
   osmosisd tx wasm migrate $CONTRACT $CODEID '{}' \
   --from= $MULTI \
   --chain-id= $CHAINID \
   --generate-only > $UNSIGNED \
   --offline \
   --sequence=$SEQUENCE \
   --account=$ACCOUNT
   ```

5. Distribute the generated file to all signers.

6. Individually sign the transaction.
   Signing over a node:

   ```
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$MULTI \
   --from=$ADDR \
   --output-document=$NAME_sig.json \
   --chain-id=$CHAINID \
   --node=$NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$MULTI \
   --from=$ADDR \
   --output-document=$NAME_sig.json \
   --chain-id=$CHAINID \
   --offline \
   --sequence=$SEQUENCE \
   --account=$ACCOUNT
   ```

7. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.
   Signing over a node:

   ```
   osmosisd tx multisign \
   $UNSIGNED \
   $MULTI \
   `$SINGER1`_sig.json `$SIGNER2`_sig.json $SIGNER3`_sig.json \
   --output-document=$SIGNED \
   --chain-id=$CHAINID \
   --node=$NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```
   osmosisd tx multisign \
   $UNSIGNED \
   $MULTI \
   `$SINGER1`_sig.json `$SIGNER2`_sig.json $SIGNER3`_sig.json \
   --output-document=$SIGNED \
   --chain-id=$CHAINID \
   --offline \
   --sequence=$SEQUENCE \
   --account=$ACCOUNT
   ```

8. Broadcast the transaction.
   ```
   osmosisd tx broadcast $SIGNED \
    --chain-id=$CHAINID \
    --broadcast-mode=block
    --node=$NODE
   ```
   Note: For the tx to be able to broadcast, the newly uploaded code needs to have a migration entry point, meaning you have to put an empty (returning Ok) migration method.

**Note:**

`CHAINID` is the id of the chain you are looking to broadcast this transaction on

- osmosis testnet - osmo-test-4
- osmosis mainnet - osmosis-1

`NODE` is the "https://rpc-" of a node on the network you want to execute the transaction

- osmosis testnet - https://rpc-test.osmosis.zone:443
- osmosis mainnet - TBD

`ACCOUNT` and `SEQUENCE` can be found by running:

```
osmosisd query account \
--node=$NODE \
--chain-id=$CHAINID \
$MULTI
```

## Signing a TX with the multisig - Execute Msg Example

1. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```
   osmosisd keys list
   ```
   
   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.
2. Initiate the multisig execute tx. This can be done by any one of the multisig holders.
   ```
   osmosisd tx wasm execute $CONTRACTADDR $ARGS \
   --from= $MULTI \
   --chain-id= $CHAINID \ 
   --generate-only > $UNSIGNED \ 
   --node=$NODE
   ```
   
3. Distribute the generated file to all signers.

4. Individually sign the transaction.
   Signing over a node:

   ```
   osmosisd tx sign \
   $UNSIGNED \
   --multisig=$MULTI \
   --from=$ADDR \
   --output-document=$NAME_sig.json \
   --chain-id=$CHAINID \
   --node=$NODE
   ```

5. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.
   Signing over a node:

   ```
   osmosisd tx multisign \
   $UNSIGNED \
   $MULTI \
   `$SINGER1`_sig.json `$SIGNER2`_sig.json $SIGNER3`_sig.json \
   --output-document=$SIGNED \
   --chain-id=$CHAINID \
   --node=$NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```
   osmosisd tx multisign \
   $UNSIGNED \
   $MULTI \
   `$SINGER1`_sig.json `$SIGNER2`_sig.json $SIGNER3`_sig.json \
   --output-document=$SIGNED \
   --chain-id=$CHAINID \
   --offline \
   --sequence=$SEQUENCE \
   --account=$ACCOUNT
   ```

6. Broadcast the transaction.
   ```
   osmosisd tx broadcast $SIGNED \
    --chain-id=$CHAINID \
    --broadcast-mode=block
    --node=$NODE
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


EXECUTE='{"set_price_source":{"denom":"uosmo","price_source":{"spot":{"pool_id":1}}}}'
osmosisd tx wasm execute $ORACLEADDR "$EXECUTE" 


EXECUTE='{"set_price_source":{"denom":"ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2","price_source":{"twap":{"pool_id":1,"window_size":86400}}}}'
osmosisd tx wasm execute $ORACLEADDR "$EXECUTE" 
```