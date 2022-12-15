# Mars Protocol
This repository contains the source code for the core smart contracts of Mars Protocol. Smart contracts are meant to be compiled to `.wasm` files and uploaded to the [Terra](https://www.terra.money/) blockchain.

## Audits
See reports [here](https://github.com/mars-protocol/mars-audits/tree/main/outposts)

## Bug bounty
A bug bounty is currently open for these contracts. See details at: https://immunefi.com/bounty/marsprotocol/

## Verify contracts
### For contracts deployed on the Osmosis chain:
1. Install Osmosisd: https://docs.osmosis.zone/osmosis-core/osmosisd/
2. Get the wasm binary executable on your local machine.
   ```shell
   git clone https://github.com/mars-protocol/outposts.git

   git checkout <commit-id>

   cd scripts

   yarn compile
   ```
   Note: Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.
3. Download the wasm from the chain.
   ```shell
   osmosisd query wasm code $CODEID -- $NODE download.wasm
   ```

4. Verify that the diff is empty between them. If any value is returned, then the wasm files differ.
   ```shell
   diff artifacts/$CONTRACTNAME.wasm download.wasm
   ```

## Environment set up
- [Install rustup](https://rustup.rs/). Once installed, make sure you have the wasm32 target:
```shell
rustup default stable
rustup update stable
rustup target add wasm32-unknown-unknown
```
- Install [cargo make](https://github.com/sagiegurari/cargo-make)

```shell
cargo install --force cargo-make
```

- Install [Docker](https://docs.docker.com/get-docker/)

- Install [Node.js v16](https://github.com/nvm-sh/nvm)

- Install [Yarn](https://classic.yarnpkg.com/lang/en/docs/install/#mac-stable)

- Create the build folder:
   ```
   yarn build
   ```
- Compile all contracts:
   ```
   cargo make rust-optimizer 
   ```
- Formatting: 
   ```
   yarn format
   
   yarn lint 
   ```
This compiles and optimizes all contracts, storing them in `/artifacts` directory along with `checksum.txt` which contains sha256 hashes of each of the `.wasm` files (The script just uses CosmWasm's [rust-optimizer](https://github.com/CosmWasm/rust-optimizer)).
Note: Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production.

## Deploying Outposts
When the deployment scripts run for the first time, it will upload code IDs for each contract, instantiate each contract, initialize assets, and set oracles. If you want to redeploy, you must locally delete the 'osmo-test-4.json' file in the artifacts directory. 
Everything related to deployment must be ran from the `scripts` directory:

Each outpost has a config file for its respective deployment and assets. 

For Osmosis:
```
cd scripts 

# for testnet deployment with deployerAddr set as owner & admin: 
yarn deploy:osmosis-testnet

# for testnet deployment with multisigAddr set as owner & admin: 
yarn deploy:osmosis-testnet-multisig 

# for mainnet deployment: 
yarn deploy:osmosis-mainnet
```

## Schemas
```
cargo make --makefile Makefile.toml generate-all-schemas
```

Creates JSON schema files for relevant contract calls, queries and query responses (See: [cosmwams-schema](https://github.com/CosmWasm/cosmwasm/tree/main/packages/schema)).

## Linting
`rustfmt` is used to format any Rust source code:

```
cargo fmt
```

`clippy` is used as a linting tool:

```
cargo +nightly clippy --tests --all-features -- -D warnings
```

## Testing
### Unit tests

```
# inside a package to run specific package tests
cargo unit-test

# in the root directory to run all tests
cargo test
```

### Integration tests

#### Running a single integration test
```
cd scripts
node --loader ts-node/esm tests/<test>.ts
```

#### Running the main integration test suite

1. Get LocalTerra repo and set `LOCAL_TERRA_REPO_PATH` env variable to its path.
2. Run `run_tests.sh` from the scripts directory:
```
cd scripts
./run_tests.sh
```

## Generating a whitelist.json

1. Create a .env file in the top level of the scripts directory if doesn't already exist
2. Add the env variable NETWORK=[network_to_generate_from_e.g._NETWORK=testnet]
3. Add the env variable REDBANK_ADDRESS=[your_deployed_red_bank_contract_address]
4. Run `node --loader ts-node/esm whitelist.ts`
5. Check the whitelists folder for [NETWORK].json output
