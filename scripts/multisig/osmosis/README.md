# Osmosis Multisig Overview

The multisig on Osmosis is set to have 5 multisig holders with a threshold of 3, meaning that 3 signitures are needed for any transaction to pass. 

## Set up Osmosisd 

Osmosisd is the daemon for the osmosis blockchain. To install, following this documentation: https://docs.osmosis.zone/osmosis-core/osmosisd/

## Set up the multisig on your local network 
_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._ 

1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.
   
   For testnet, go to: 
    
   ```https://lcd-test.osmosis.zone/cosmos/auth/v1beta1/accounts/INSERT_YOUR_WALLET_ADDRESS```

    For mainnet, go to: 
   
    ```https://osmosis-api.polkachu.com/cosmos/auth/v1beta1/accounts/INSERT_YOUR_WALLET_ADDRESS```
    
    These websites will return a JSON that has your pubkey. Copy your pubkey in the following format: 
    ```
   '{
    "@type": "/cosmos.crypto.secp256k1.PubKey",
    "key": "alkfjadfyeohiskvbskjas,jdla"
    }'
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
5. Update the config with the new mutlisig address in ```outposts/scripts/deploy/osmosis/config```, which will set the owner and admin of the smart contracts to the multisig upon deployment. 

## Signing a TX with the multisig - Migrate Msg Example
_Note: The multisig must have at least one tx against it for the address to exist in Osmosis' state._ 

1. If the multisig has no txs against it, send some tokens to the account. 

2. Assert that you have both your own wallet and multisig wallet in your keyring. 
   ```
   osmosisd keys list
   ```
   If they're missing, follow steps 2-4 from the "Set up multisig on your local network" section.

3. Initiate the multisig tx. This can be done by any one of the multisig holders. 
   
   Signing over a node: 
   ```
   osmosisd tx wasm migrate [contract address] [new_code_id] '{}' 
   --from [multisig_address]
   --chain-id [chain_id] 
   --generate-only > [unsignedTx_filename].json
   --node=[node_address]
   ```
   Or do an offline sign mode: 
   
   _Recommended when signing many transactions in a sequence before they are executed._
   ```
   osmosisd tx wasm migrate [contract address] [new_code_id] '{}' 
   --from [multisig_address]
   --chain-id [chain_id] 
   --generate-only > [unsignedTx_filename].json
   --offline
   --sequence=[current_account_sequence]
   --account=[acount_number] 
   ```
4. Distribute the ```[unsignedTx_filename].json``` file to all signers. 

5. Individually sign the transaction.
   Signing over a node:
   ```
   osmosisd tx sign \
    [unsignedTx_filename].json \
    --multisig=[multisig_address] \
    --from=[your_wallet_address] \
    --output-document=[name]sig.json \
    --chain-id=[chain_id]
   --node=[node_address]
   ```
   Or do an offline sign mode: 

   _Recommended when signing many transactions in a sequence before they are executed._
   ```
   osmosisd tx sign \
    [unsignedTx_filename].json \
    --multisig=[multisig_address] \
    --from=[your_wallet_address] \
    --output-document=[name]sig.json \
    --chain-id=[chain_id]
    --offline 
    --sequence=[current account sequence] 
    --account=[account number] 
   ```

6. Complete the multisign. There must be a total of 3 signers for the transaction to be successful.
   Signing over a node:
   ```
   osmosisd tx multisign \
    [unsignedTx_filename].json \
    [multisig_address] \
    [name1]sig.json [name2]sig.json [name3]sig.json \
    --output-document=[signedTx_filename].json \
    --chain-id=[chain_id]
    --node=[node_address]
   ```
   Or do an offline sign mode: 

   _Recommended when signing many transactions in a sequence before they are executed._
   ```
   osmosisd tx multisign \
   [unsignedTx_filename].json \
   [multisig_address] \
   [name1]sig.json [name2]sig.json [name3]sig.json \
   --output-document=[signedTx_filename].json \
   --chain-id=[chain_id]
   --offline
   --sequence=[current_account_sequence]
   --account=[acount_number] 
   ```
7. Broadcast the transaction. 
   ```
   osmosisd tx broadcast [signedTx_filename].json \
    --chain-id=[chain_id] \
    --broadcast-mode=block
    --node=https://rpc-test.osmosis.zone:443
   ```

**Note:** 

```chain_id``` is the id of the chain you are looking to broadcast this transaction on 
   * osmosis testnet - osmo-test-4
   * osmosis mainnet - osmosis-1

```node_address``` is the "https://rpc-" of a node on the network you want to execute the transaction
   * osmosis testnet - https://rpc-test.osmosis.zone:443
   * osmosis mainnet - TBD

```account_number``` and ```sequence_number``` can be found by running: 
   ```
   osmosisd query account \
   --node=[node_address] \
   --chain-id=[chain_id] \
   [multisig_address]
   ```
For the multisig address osmo1jklpvl3446z5qw58cvq8hqvthzjtsfvs9j65tq: 
* sequence = 0 
* account number = 274573

## Signing a TX with the multisig - Execute Msg Example