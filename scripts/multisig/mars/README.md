# Mars Hub Multisig

The multisig on Mars Hub is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

## Installing marsd

1. Install homebrew: <https://brew.sh/>

2. Clone the following repository: <https://github.com/mars-protocol/hub/tags>

3. Check out to the latest stable release:

   ```bash
   git checkout <tag>
   ```

4. `make install`

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._

1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

   ```bash
   marsd query account [address] --node=[node_URL]
   ```

2. Add each public key to the keys list in your local network.

   ```bash
   marsd keys add [name] --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.

   ```bash
   marsd keys add mars_multisig \
     --multisig=[name1],[name2],[name3],[name4],[name5] \
     --multisig-threshold=3
   ```

4. Assert that it was completed correctly.

   ```bash
   marsd keys show mars_multisig
   ```

## Set up environment variables

These variables change based on the network, transaction, time, and user. Therefore, they should be provided to the multisig holders before each transaction and updated as needed on your machine.

For `bash`:

```bash
# Mars Testnet variables
export MARS_MULTISIG="mars1skwmcsesjj99hye93smjz88rh0qndhvahewr60"
export MARS_TEST_NODE="https://testnet-rpc.marsprotocol.io:443"
export MARS_TEST_VESTING="mars14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9smxjtde"
export MARS_TEST_AIRDROP="mars1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqhnhf0l"
export MARS_TEST_DELEGATOR="mars17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgs0gfvxm"
export MARS_TEST_CHAIN_ID="ares-1"

# Transaction specific variables (must be created at time of transaction)
export CODEID="new_code_ID_to_migrate_to"
export MARS_SEQUENCE="current_account_sequence"
export UNSIGNED="unsignedTX_filename.JSON"
export SIGNEDTX="signedTX_filenme.JSON"
export EXECUTE="msg_to_execute"

# User specific variables
export SINGLE_SIGN="your_name.JSON"
export MARS_ADDR="your_wallet_address"
```

**Note:** `MARS_ACCOUNT` and `MARS_SEQUENCE` can be found by running:

```bash
marsd query account $MARS_MULTI \
  --node=$MARS_TEST_NODE \
  --chain-id=$MARS_TEST_CHAINID
```

## Verifying contracts

1. Get the wasm binary executable on your local machine.

   ```bash
   git clone https://github.com/mars-protocol/periphery
   git checkout <commit-id>
   cargo make rust-optimizer
   ```

   Note: Intel/AMD 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.

2. Download the wasm from the chain.

   ```bash
   marsd query wasm code $CODEID --$NODE download.wasm
   ```

3. Verify that the diff is empty between them. If any value is returned, then the wasm files differ.

   ```bash
   diff artifacts/$CONTRACTNAME.wasm download.wasm
   ```

## Query contract configs

- Airdrop Contract Config:

  ```shell
  QUERY='{"config":{}}'
  marsd query wasm contract-state smart $MARS_TEST_AIRDROP "$QUERY" --output json --node=$MARS_TEST_NODE
  ```

- Vesting Config:

  ```shell
  QUERY='{"config":{}}'
  marsd query wasm contract-state smart $MARS_TEST_VESTING "$QUERY" --output json --node=$MARS_TEST_NODE
  ```

- Delegator Config:

  ```shell
  QUERY='{"config":{}}'
  marsd query wasm contract-state smart $MARS_TEST_DELEGATOR "$QUERY" --output json --node=$MARS_TEST_NODE
  ```

## Signing a tx with the multisig - testnet migrate msg example

**Every multisig holder is responsible for verifying the contract's newly uploaded code for every migrate msg.**

_Note: The multisig must have at least one tx against it for the address to exist in Mars' state._

1. If the multisig has no txs against it, send some tokens to the account. Otherwise, the account does not exist in Mars' state.

2. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```bash
   marsd keys list
   ```

   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.

3. Ensure the newly uploaded code has a migration entry point.

   ```rust
   use cosmwasm_schema::cw_serde;
   use cosmwasm_std::{entry_point, DepsMut, Env, Response, StdResult};

   #[cw_serde]
   struct MigrateMsg {}

   #[entry_point]
   fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
       Ok(Response::default())
   }
   ```

4. Initiate the multisig migrate tx. This can be done by any one of the multisig holders.

   Signing over a node:

   ```bash
   marsd tx wasm migrate $CONTRACT $CODEID '{}' \
     --from=$MARS_MULTI \
     --chain-id=$MARS_TEST_CHAINID \
     --generate-only > $UNSIGNED \
     --node=$MARS_TEST_NODE
   ```

   Or do an offline sign mode:

   _Recommended when signing many transactions in a sequence before they are executed._

   ```bash
   marsd tx wasm migrate $CONTRACT $CODEID '{}' \
     --from=$MARS_MULTI\
     --chain-id=$MARS_TEST_CHAINID \
     --generate-only > $UNSIGNED \
     --offline \
     --sequence=$MARS_SEQUENCE \
     --account-number=$MARS_ACCOUNT
   ```

5. Distribute the generated file to all signers.

6. Individually sign the transaction.
   Signing over a node:

   ```bash
   marsd tx sign $UNSIGNED \
     --multisig=$MARS_MULTI \
     --from=$MARS_ADDR \
     --output-document=$SINGLE_SIGN \
     --chain-id=$MARS_TEST_CHAINID \
     --node=$MARS_TEST_NODE

   ## When using a ledger:
     --sign-mode=amino-json
   ```

7. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.
   Signing over a node:

   ```bash
   marsd tx multisign $UNSIGNED $MARS_MULTI `$SINGER1`.json `$SIGNER2`.json `$SIGNER3`.json \
     --output-document=$SIGNED \
     --chain-id=$MARS_TEST_CHAINID \
     --node=$MARS_TEST_NODE
   ```

8. Broadcast the transaction.

   ```bash
   marsd tx broadcast $SIGNED \
     --chain-id=$MARS_TEST_CHAINID \
     --broadcast-mode=block
     --node=$MARS_TEST_NODE
   ```

   Note: For the tx to be able to broadcast, the newly uploaded code needs to have a migration entry point, meaning you have to put an empty (returning Ok) migration method.

9. Verify the new contract. Get the wasm binary executable on your local machine.

   ```bash
   git clone https://github.com/mars-protocol/periphery
   git checkout <commit-id>
   ```

   ```bash
   docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/workspace-optimizer:0.12.10
   ```

   Note: Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.

   Download the wasm from the chain.

   ```bash
   marsd query wasm code $CODEID --$NODE download.wasm
   ```

   Verify that the diff is empty between them. If any value is returned, then the wasm files differ.

   ```bash
   diff artifacts/$CONTRACTNAME.wasm download.wasm
   ```

## Signing a tx with the multisig - testnet execute msg example

**Every multisig holder is responsible for verifying the contract's newly uploaded code for every migrate msg.**

_Note: The multisig must have at least one tx against it or be registered in the auth module for the address to exist in Mars' state._

1. If the multisig does not exist in Mars' state, send some tokens to the account. Otherwise, the account cannot run the following commands.

2. Assert that you have both your own wallet and multisig wallet in your keyring.

   ```bash
   marsd keys list
   ```

   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.

3. Initiate the multisig execute tx. This can be done by any one of the multisig holders.

   ```bash
   marsd tx wasm execute $CONTRACTADDR $EXECUTE \
     --from=$MARS_MULTI \
     --chain-id=$MARS_TEST_CHAINID \
     --generate-only > $UNSIGNED \
     --node=$MARS_TEST_NODE
   ```

4. Distribute the generated file to all signers.

5. Individually sign the transaction.

   ```bash
   marsd tx sign $UNSIGNED \
     --multisig=$MARS_MULTI \
     --from=$MARS_ADDR \
     --output-document=$SINGLE_SIGN \
     --chain-id=$MARS_TEST_CHAINID \
     --node=$MARS_TEST_NODE
   ```

6. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.

   ```bash
   marsd tx multisign $UNSIGNED $MARS_MULTI `$SINGER1`.json `$SIGNER2`.json `$SIGNER3`.json \
     --output-document=$SIGNED \
     --chain-id=$OSMO_TEST_CHAINID \
     --node=$MARS_TEST_NODE
   ```

7. Broadcast the transaction.

   ```bash
   marsd tx broadcast $SIGNED \
     --chain-id=$MARS_TEST_CHAINID \
     --broadcast-mode=block
     --node=$MARS_TEST_NODE
   ```

## Examples of execute args

For this to be completed as a multisig tx, the flags and steps from the previous section must be used.

```bash
# VESTING
EXECUTE='{"create_position":{"user":"$ADDR,"vest_schedule":"$VEST_SCHEDULE}}'
marsd tx wasm execute $MARS_TEST_VESTING "$EXECUTE"
```
