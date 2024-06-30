# Neutron Multisig Overview

The multisig on Neutron is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

## Set up Neutrond

Neutrond is the daemon for the neutron blockchain. To install, follow [this documentation](https://docs.neutron.org/neutron/build-and-run/neutron-build).

## Set up individual account as a multisig signer on your local network

1. Create the account - to use a consistent naming, we will use [name]\_ntrn e.g. dane_ntrn (similarly on other chains e.g. dane_osmo, dane_mars). It is up to the signer if they wish to use a Ledger or other hardware wallet - or not.

The benefit is that you will be more secure of a signer
The downsides are that:
a. Some Ledgers are not able to sign large messages such as contract uploads
b. If you are traveling a lot it's best to leave your hardware wallet at home in a secure place, and so if this is the case it might actually be more secure to have a hot wallet as hardware wallets are easily recognisable in airport security etc.

```bash
neutrond keys add [name]_ntrn
```

2. Note down the mnemonic - it is important that you are able to recover this account as a multisig signer.

3. Send a small amount of funds to the address to register it. In testnet you can do this by visiting the faucet [here](https://t.me/+SyhWrlnwfCw2NGM6)

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._

1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

To do a send transaction of 1 NTRN to another account you can use the command:

```bash
neutrond tx bank send [name]_ntrn [to_address] 1000000untrn --node=[rpc node] --chain-id=[chain id]
```

Note for testnet node you can use https://testnet-neutron-rpc.marsprotocol.io:443 and chain-id pion-1

Query the public key:

```bash
neutrond query account [address] --node=[node_URL]
```

2. Add each public key to the keys list in your local network.

   ```bash
   neutrond keys add [name]_ntrn --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.

   ```bash
   neutrond keys add neutron_multisig \
     --multisig=[name1],[name2],[name3],[name4],[name5] \
     --multisig-threshold=3
   ```

4. Assert that it was completed correctly.

   ```bash
   neutrond keys show neutron_multisig
   ```

5. Update the config with the new multisig address in `red-bank/scripts/deploy/neutron/config`, which will set the owner and admin of the smart contracts to the multisig upon deployment.

## Set up environment variables

These variables change based on the network, transaction, time, and user. Therefore, they should be provided to the multisig holders before each transaction and updated as needed on your machine.

For `# bash`:

```bash
# Neutron Mainnet variables
export NEUTRON_MULTI="neutron1ltzuv25ltw9mkwuvvmt7e54a6ene283hfj7l0c"
export NEUTRON_CHAINID="neutron-1"
export NEUTRON_NODE="https://neutron.rpc.p2p.world:443/qgrnU6PsQZA8F9S5Fb8Fn3tV3kXmMBl2M9bcc9jWLjQy8p"
export NEUTRON_ACCOUNT="51587"
export NEUTRON_ADDR_PROVIDER="contract_address_here_once_deployed"
export NEUTRON_REDBANK="contract_address_here_once_deployed"
export NEUTRON_INCENTIVES="contract_address_here_once_deployed"
export NEUTRON_ORACLE="contract_address_here_once_deployed"
export NEUTRON_REWARDS_COLLECTOR="contract_address_here_once_deployed"
export NEUTRON_SWAPPER="contract_address_here_once_deployed"
export NEUTRON_LIQUIDATION_FILTERER="contract_address_here_once_deployed"

# Transaction specific variables (must be created at time of transaction)
export CODEID="new_code_ID_to_migrate_to"
export SEQUENCE="current_account_sequence"
export UNSIGNED="unsignedTX_filename.JSON"
export SIGNEDTX="signedTX_filenme.JSON"
export EXECUTE="msg_to_execute"

# User specific variables
export SINGLE_SIGN="your_name.JSON"
export NEUTRON_ADDR="your_wallet_address"
```

**Note:**

`NEUTRON_ACCOUNT` and `SEQUENCE` can be found by running:

```bash
neutrond query account \
--node=$NEUTRON_NODE \
--chain-id=$NEUTRON_CHAINID \
$NEUTRON_MULTI
```

## Verifying Contracts

1. Get the wasm binary executable on your local machine.

   For address-provider, incentives, oracle, red-bank, rewards-collector, swapper contracts:

   ```bash
   git clone https://github.com/mars-protocol/red-bank.git
   git checkout <commit-id>
   cargo make rust-optimizer
   ```

   For liquidation-filterer contract

   ```bash
   git clone https://github.com/mars-protocol/liquidation-helpers
   git checkout <commit-id>
   cargo make rust-optimizer
   ```

   Note: Intel/AMD 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.

2. Download the wasm from the chain.

   ```bash
   neutrond query wasm code $CODEID -- $NODE download.wasm
   ```

3. Verify that the diff is empty between them. If any value is returned, then the wasm files differ.

   ```bash
   diff artifacts/$CONTRACTNAME.wasm download.wasm
   ```

## Query contract configs

- Red Bank Contract Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_REDBANK "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Oracle Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_ORACLE "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Incentives Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_INCENTIVES "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Address Provider Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_ADDR_PROVIDER "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Rewards Collector Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_REWARDS_COLLECTOR "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Swapper Config:

  ```bash
  QUERY='{"owner": {}}'
  neutrond query wasm contract-state smart $NEUTRON_SWAPPER "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Liquidation Filterer Config:

  ```bash
  QUERY='{"config": {}}'
  neutrond query wasm contract-state smart $NEUTRON_LIQUIDATION_FILTERER "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Verify NTRN, ATOM, and axlUSDC are initialized in the red bank market and have the correct params:

  ```bash
  QUERY='{"market":{"denom":"utrn"}}'
  neutrond query wasm contract-state smart $NEUTRON_REDBANK "$QUERY" --output json --node=$NEUTRON_NODE

  QUERY='{"market":{"denom":"ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9"}}'
  neutrond query wasm contract-state smart $NEUTRON_REDBANK "$QUERY" --output json --node=$NEUTRON_NODE

  QUERY='{"market":{"denom":"ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349"}}'
  neutrond query wasm contract-state smart $NEUTRON_REDBANK "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Verify Oracle Price Source is set correctly:

  ```bash
  QUERY='{"price_sources":{}}'
  neutrond query wasm contract-state smart $NEUTRON_ORACLE "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Verify Swaper Routes are set correctly:

  ```bash
  QUERY='{"routes":{}}'
  neutrond query wasm contract-state smart $NEUTRON_SWAPPER "$QUERY" --output json --node=$NEUTRON_NODE
  ```

- Verify Admin is set correctly:

  _Note: If admin is not set, contracts are immutable_

  ```bash
  neutrond query wasm contract $NEUTRON_REWARDS_COLLECTOR --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_RED_BANK --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_ADDR_PROVIDER --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_ORACLE --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_INCENTIVES --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_SWAPPER --node=$NEUTRON_NODE
  neutrond query wasm contract $NEUTRON_LIQUIDATION_FILTERER --node=$NEUTRON_NODE
  ```

## Signing a TX with the multisig - Testnet Migrate Msg Example

**Every multisig holder is responsible for verifying the contract's newly uploaded code for every migrate msg.**

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond

## Signing a TX with the multisig - Testnet Execute Msg Example

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond

## Examples of Execute Args

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond
