# Mars Protocol
This repository contains the source code for the core smart contracts of Mars Protocol. Smart contracts are meant to be compiled to `.wasm` files and uploaded to the [Terra](https://www.terra.money/) blockchain.

## Bug bounty
A bug bounty is currently open for these contracts. See details at: https://immunefi.com/bounty/marsprotocol/

## Verify contracts

Follow these instructions to verify that the smart contracts that exist on chain correspond to a particular version of the contract's source code:

1. Find the code ID of the contract you wish to verify.

    This can be found on the smart contract's page on [Terra Finder](https://finder.terra.money/).
    
2. Get the SHA256 checksum of the code ID's wasm binary:
    - One way to do this is to get the checksum directly from the blockchain:

    ```
    curl "https://fcd.terra.dev/terra/wasm/v1beta1/codes/${CODE_ID}" \
      | jq ".code_info.code_hash" \
      | tr -d \" \
      | base64 -d \
      | hexdump -v -e '/1 "%02x"'
    ```

    - Alternatively, download the wasm byte code relating to the code ID from the blockchain and calculate its SHA256 checksum:

    ```
    curl "https://fcd.terra.dev/terra/wasm/v1beta1/codes/${CODE_ID}/byte_code" \
      | jq ".byte_code" \
      | tr -d \" \
      | base64 -d \
      | shasum -a 256
    ```

3. Get the SHA256 checksum of a smart contract's wasm binary built from source code. To do this, first clone this repo, checkout a particular release, compile the smart contracts using the same version of [rust-optimizer](https://github.com/CosmWasm/rust-optimizer) listed in the [releases](https://github.com/mars-protocol/mars-core/releases), and verify the checksum written to `artifacts/checksums.txt`.
4. Finally, verify that the two checksums are identical.

## Building

### Smart contracts
```
./scripts/build_artifacts.sh
```

Compiles and optimizes all contracts, storing them in `/artifacts` directory along with `checksum.txt` which contains sha256 hashes of each of the `.wasm` files (The script just uses CosmWasm's [rust-optimizer](https://github.com/CosmWasm/rust-optimizer)).

### Schemas
```
./scripts/build_schema.sh
```

Is equivalent of running `cargo schema` in each of the smart contracts in `/contracts` directory, which creates JSON schema files for relevant contract calls, quieries and query responses (See: [cosmwams-schema](https://github.com/CosmWasm/cosmwasm/tree/main/packages/schema).

## Linting
`rustfmt` is used to format any Rust source code:

```
cargo fmt
```

`clippy` is used as a linting tool:

```
cargo +nightly clippy --tests --all-features -- -D warnings
```

## Setup for Node.js scripts

Node.js scripts are used for deploying and integration testing. These scripts must be run from the `scripts` directory.

Setup:

```
cd scripts
npm install
```

TypeScript scripts must be executed with `ts-node` using:

```
node --loader ts-node/esm <script>.ts
```


## Deploying
```
# build the smart contracts
./scripts/build_artifacts.sh

cd scripts
npm install

# set the deploying wallet
echo "TEST_MAIN=<MNEMONIC_OF_YOUR_DEPLOYING_WALLET>" >> .env

# set the network, defaults to LocalTerra if unset
echo "NETWORK=testnet" >> .env

# ensure the deploy_config.ts has a cw20_code_id specified for above network

node --loader ts-node/esm deploy.ts
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

#### Local Terra
Integration tests require LocalTerra to be running:

```
git clone https://github.com/terra-money/LocalTerra.git
cd LocalTerra
docker compose up
```

Adjust the `timeout_*` config items in `LocalTerra/config/config.toml` to `250ms` to make the test run faster:

```
sed -E -I .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/250ms/' config/config.toml
```

#### Environment variables
Required environment variables (can be set in `scripts/.env`):

```sh
CW_PLUS_ARTIFACTS_PATH # path to cw-plus artifacts (example cw20-base.wasm)
ASTROPORT_ARTIFACTS_PATH # path to astroport artifacts
MARS_MOCKS_ARTIFACTS_PATH # path to mars-mocks artifacts
```

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

## License

Contents of this repository are open source under [GNU General Public License v3](https://www.gnu.org/licenses/gpl-3.0.en.html) or later.
