# Mars Protocol: Red Bank

This repository contains the source code for the core smart contracts of Mars Red Bank. Smart contracts are meant to be compiled to `.wasm` files and uploaded to the Cosmos chains.

## Audits

See reports [here][1].

## Bug bounty

A bug bounty is currently open for these contracts. See details [here][2].

## Verify contracts

### For contracts deployed on the Osmosis chain

1. Install [Osmosisd][3]

2. Get the wasm binary executable on your local machine.

   ```bash
   git clone https://github.com/mars-protocol/outposts.git
   git checkout <commit-id>
   cargo make rust-optimizer
   ```

   Note: Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and the wasm bytecode will not match up to an Intel compiled wasm file.

3. Download the wasm from the chain.

   ```bash
   osmosisd query wasm code $CODEID -- $NODE download.wasm
   ```

4. Verify that the diff is empty between them. If any value is returned, then the wasm files differ.

   ```bash
   diff artifacts/$CONTRACTNAME.wasm download.wasm
   ```

5. Alternatively, compare the wasm files' checksums:

   ```bash
   sha256sum artifacts/$CONTRACTNAME.wasm download.wasm
   ```

## Environment set up

- Install [rustup][4]. Once installed, make sure you have the wasm32 target:

  ```bash
  rustup default stable
  rustup update stable
  rustup target add wasm32-unknown-unknown
  ```

- Install [cargo-make][5]

  ```bash
  cargo install --force cargo-make
  ```

- Install [Docker][6]

- Install [Node.js v16][7]

- Install [Yarn][8]

- Create the build folder:

   ```bash
   yarn build
   ```

- Compile all contracts:

   ```bash
   cargo make rust-optimizer
   ```

- Formatting:

   ```bash
   yarn format
   yarn lint
   ```

This compiles and optimizes all contracts, storing them in `/artifacts` directory along with `checksum.txt` which contains sha256 hashes of each of the `.wasm` files (The script just uses CosmWasm's [rust-optimizer][9]).

**Note:** Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production.

## Deployment

When the deployment scripts run for the first time, it will upload code IDs for each contract, instantiate each contract, initialize assets, and set oracles. If you want to redeploy, you must locally delete the `osmo-test-4.json` file in the artifacts directory.

Everything related to deployment must be ran from the `scripts` directory.

Each outpost has a config file for its respective deployment and assets.

For Osmosis:

```bash
cd scripts

# for testnet deployment with deployerAddr set as owner & admin:
yarn deploy:osmosis-testnet

# for testnet deployment with multisigAddr set as owner & admin:
yarn deploy:osmosis-testnet-multisig

# for mainnet deployment:
yarn deploy:osmosis-mainnet
```

## Schemas

```bash
cargo make --makefile Makefile.toml generate-all-schemas
```

Creates JSON schema files for relevant contract calls, queries and query responses (See: [cosmwams-schema][10]).

## Linting

`rustfmt` is used to format any Rust source code:

```bash
cargo fmt
```

`clippy` is used as a linting tool:

```bash
cargo make clippy
```

## Testing

Integration tests (task `integration-test` or `test`) use `.wasm` files. They have to be generated with `cargo make build`.

Run unit tests:

```bash
cargo make unit-test
```

Run integration tests:

```bash
cargo make integration-test
```

Run all tests:

```bash
cargo make test
```

## Deployments

### osmosis-1

TBD

### osmo-test-4

| Contract               | Address                                                           |
| ---------------------- | ----------------------------------------------------------------- |
| mars-address-provider  | `osmo17dyy6hyzzy6u5khy5lau7afa2y9kwknu0aprwqn8twndw2qhv8ls6msnjr` |
| mars-incentives        | `osmo1zxs8fry3m8j94pqg7h4muunyx86en27cl0xgk76fc839xg2qnn6qtpjs48` |
| mars-oracle            | `osmo1dqz2u3c8rs5e7w5fnchsr2mpzzsxew69wtdy0aq4jsd76w7upmsstqe0s8` |
| mars-red-bank          | `osmo1t0dl6r27phqetfu0geaxrng0u9zn8qgrdwztapt5xr32adtwptaq6vwg36` |
| mars-rewards-collector | `osmo14kzsqw5tatdvwlkj383lgkh6gcdetwn7kfqm7488uargyy2lpucqsyv53j` |

## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.

[1]: https://github.com/mars-protocol/mars-audits/tree/main/outposts
[2]: https://immunefi.com/bounty/mars/
[3]: https://docs.osmosis.zone/osmosis-core/osmosisd/
[4]: https://rustup.rs/
[5]: https://github.com/sagiegurari/cargo-make
[6]: https://docs.docker.com/get-docker/
[7]: https://github.com/nvm-sh/nvm
[8]: https://classic.yarnpkg.com/lang/en/docs/install/#mac-stable
[9]: https://github.com/CosmWasm/rust-optimizer
[10]: https://github.com/CosmWasm/cosmwasm/tree/main/packages/schema
