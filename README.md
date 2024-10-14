# Smart contracts for Mars Outposts

This repository contains the source code for the core smart contracts of Mars Protocol. Smart contracts are meant to be compiled to `.wasm` files and uploaded to the Cosmos chains.

## Audits

See reports for red-bank and rover [here][1].

## Bug bounty

A bug bounty is currently open for these contracts. See details [here][2].

## Verify contracts

### For contracts deployed on the Osmosis chain

1. Install [Osmosisd][3]

2. Get the wasm binary executable on your local machine.

   ```bash
   git clone https://github.com/mars-protocol/contracts.git
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

- Install [cargo-make][4]

  ```bash
  cargo install --force cargo-make
  ```

- Install [rust][5]

  ```bash
  cargo make install-stable
  ```

- Install [Docker][6]

- Install [Node.js v16][7]

- Install [Yarn][8]

- Create the build folder:

   ```bash
   cd scripts
   yarn
   yarn build
   ```

- Compile all contracts:

  ```bash
  cargo make rust-optimizer
  ```

- Formatting:

   ```bash
   cd scripts
   yarn format
   yarn lint
   ```

This compiles and optimizes all contracts, storing them in `/artifacts` directory along with `checksum.txt` which contains sha256 hashes of each of the `.wasm` files (The script just uses CosmWasm's [rust-optimizer][9]).

**Note:** Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production.

## Deployment

When the deployment scripts run for the first time, it will upload code IDs for each contract, instantiate each contract, initialize assets, and set oracles. If you want to redeploy, you must locally delete the file with `.json` extension (e.g. `devnet-deployer-owner.json`) in the artifacts directory.

Everything related to deployment must be ran from the `scripts` directory.

Each outpost has a config file for its respective deployment and assets.

For Osmosis:

```bash
cd scripts

# for devnet deployment with deployerAddr set as owner & admin:
yarn deploy:osmosis-devnet

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
cargo make fmt
```

`clippy` is used as a linting tool:

```bash
cargo make clippy
```

## Testing

Install [Go][38]. It is used by [osmosis-test-tube][39] dependency.

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

| Contract               | Address                                                                 | Tag
| ---------------------- | ----------------------------------------------------------------------- | --------------
| mars-address-provider  | [`osmo1g677w7mfvn78eeudzwylxzlyz69fsgumqrscj6tekhdvs8fye3asufmvxr`][11] | [`v2.0.0`][40] |
| mars-account-nft       | [`osmo1450hrg6dv2l58c0rvdwx8ec2a0r6dd50hn4frk370tpvqjhy8khqw7sw09`][12] | [`v2.0.0`][43] |
| mars-credit-manager    | [`osmo1f2m24wktq0sw3c0lexlg7fv4kngwyttvzws3a3r3al9ld2s2pvds87jqvf`][13] | [`v2.0.5`][49] |
| mars-health            | [`osmo1pdc49qlyhpkzx4j24uuw97kk6hv7e9xvrdjlww8qj6al53gmu49sge4g79`][14] | [`v2.0.0`][43] |
| mars-incentives        | [`osmo1nkahswfr8shg8rlxqwup0vgahp0dk4x8w6tkv3rra8rratnut36sk22vrm`][15] | [`v2.0.0`][40] |
| mars-oracle            | [`osmo1mhznfr60vjdp2gejhyv2gax9nvyyzhd3z0qcwseyetkfustjauzqycsy2g`][16] | [`v2.0.1`][41] |
| mars-params            | [`osmo1nlmdxt9ctql2jr47qd4fpgzg84cjswxyw6q99u4y4u4q6c2f5ksq7ysent`][17] | [`v2.0.3`][46] |
| mars-red-bank          | [`osmo1c3ljch9dfw5kf52nfwpxd2zmj2ese7agnx0p9tenkrryasrle5sqf3ftpg`][18] | [`v2.0.5`][49] |
| mars-rewards-collector | [`osmo1urvqe5mw00ws25yqdd4c4hlh8kdyf567mpcml7cdve9w08z0ydcqvsrgdy`][19] | [`v2.0.4`][47] |
| mars-swapper           | [`osmo1wee0z8c7tcawyl647eapqs4a88q8jpa7ddy6nn2nrs7t47p2zhxswetwla`][20] | [`v2.0.7`][50] |
| mars-zapper            | [`osmo17qwvc70pzc9mudr8t02t3pl74hhqsgwnskl734p4hug3s8mkerdqzduf7c`][21] | [`v2.0.0`][43] |

### neutron-1

| Contract                  | Address                                                                    | Tag
| ------------------------- | -------------------------------------------------------------------------- | --------------
| mars-address-provider     | [`neutron17yehp4x7n79zq9dlw4g7xmnrvwdjjj2yecq26844sg8yu74knlxqfx5vqv`][24] | [`v2.1.0`][57] |
| mars-account-nft          | [`neutron184kvu96rqtetmunkkmhu5hru8yaqg7qfhd8ldu5avjnamdqu69squrh3f5`][52] | [`v2.1.0`][57] |
| mars-credit-manager       | [`neutron1qdzn3l4kn7gsjna2tfpg3g3mwd6kunx4p50lfya59k02846xas6qslgs3r`][53] | [`v2.1.0`][57] |
| mars-health               | [`neutron17ktfwsr7ghlxzzma0gw0hke3j3rnssd58q87jv2wzfrk6uhawa3sv8xxtm`][54] | [`v2.1.0`][57] |
| mars-incentives           | [`neutron1aszpdh35zsaz0yj80mz7f5dtl9zq5jfl8hgm094y0j0vsychfekqxhzd39`][25] | [`v2.1.0`][57] |
| mars-oracle               | [`neutron1dwp6m7pdrz6rnhdyrx5ha0acsduydqcpzkylvfgspsz60pj2agxqaqrr7g`][26] | [`v2.1.0`][57] |
| mars-params               | [`neutron1x4rgd7ry23v2n49y7xdzje0743c5tgrnqrqsvwyya2h6m48tz4jqqex06x`][55] | [`v2.1.0`][57] |
| mars-red-bank             | [`neutron1n97wnm7q6d2hrcna3rqlnyqw2we6k0l8uqvmyqq6gsml92epdu7quugyph`][27] | [`v2.1.0`][57] |
| mars-rewards-collector    | [`neutron1h4l6rvylzcuxwdw3gzkkdzfjdxf4mv2ypfdgvnvag0dtz6x07gps6fl2vm`][28] | [`v2.1.0`][57] |
| mars-swapper              | [`neutron1udr9fc3kd743dezrj38v2ac74pxxr6qsx4xt4nfpcfczgw52rvyqyjp5au`][29] | [`v2.1.0`][57] |
| mars-zapper               | [`neutron1dr0ckm3u2ztjuscmgqjr85lwyduphxkgl3tc02ac8zp54r05t5dqp5tgyq`][56] | [`v2.1.0`][57] |

### mars-1

| Module Account  | Address                                             |
| --------------- | --------------------------------------------------- |
| `fee_collector` | [`mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x`][36] |
| `safety`        | [`mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575`][37] |

## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.

[1]: https://github.com/mars-protocol/mars-audits
[2]: https://immunefi.com/bounty/mars/
[3]: https://docs.osmosis.zone/osmosis-core/osmosisd/
[4]: https://github.com/sagiegurari/cargo-make
[5]: https://rustup.rs/
[6]: https://docs.docker.com/get-docker/
[7]: https://github.com/nvm-sh/nvm
[8]: https://classic.yarnpkg.com/lang/en/docs/install/#mac-stable
[9]: https://github.com/CosmWasm/rust-optimizer
[10]: https://github.com/CosmWasm/cosmwasm/tree/main/packages/schema
[11]: https://osmosis.celat.one/osmosis-1/contracts/osmo1g677w7mfvn78eeudzwylxzlyz69fsgumqrscj6tekhdvs8fye3asufmvxr
[12]: https://osmosis.celat.one/osmosis-1/contracts/osmo1450hrg6dv2l58c0rvdwx8ec2a0r6dd50hn4frk370tpvqjhy8khqw7sw09
[13]: https://osmosis.celat.one/osmosis-1/contracts/osmo1f2m24wktq0sw3c0lexlg7fv4kngwyttvzws3a3r3al9ld2s2pvds87jqvf
[14]: https://osmosis.celat.one/osmosis-1/contracts/osmo1pdc49qlyhpkzx4j24uuw97kk6hv7e9xvrdjlww8qj6al53gmu49sge4g79
[15]: https://osmosis.celat.one/osmosis-1/contracts/osmo1nkahswfr8shg8rlxqwup0vgahp0dk4x8w6tkv3rra8rratnut36sk22vrm
[16]: https://osmosis.celat.one/osmosis-1/contracts/osmo1mhznfr60vjdp2gejhyv2gax9nvyyzhd3z0qcwseyetkfustjauzqycsy2g
[17]: https://osmosis.celat.one/osmosis-1/contracts/osmo1nlmdxt9ctql2jr47qd4fpgzg84cjswxyw6q99u4y4u4q6c2f5ksq7ysent
[18]: https://osmosis.celat.one/osmosis-1/contracts/osmo1c3ljch9dfw5kf52nfwpxd2zmj2ese7agnx0p9tenkrryasrle5sqf3ftpg
[19]: https://osmosis.celat.one/osmosis-1/contracts/osmo1urvqe5mw00ws25yqdd4c4hlh8kdyf567mpcml7cdve9w08z0ydcqvsrgdy
[20]: https://osmosis.celat.one/osmosis-1/contracts/osmo1wee0z8c7tcawyl647eapqs4a88q8jpa7ddy6nn2nrs7t47p2zhxswetwla
[21]: https://osmosis.celat.one/osmosis-1/contracts/osmo17qwvc70pzc9mudr8t02t3pl74hhqsgwnskl734p4hug3s8mkerdqzduf7c
[22]: https://osmosis.celat.one/osmosis-1/contracts/osmo1kqzkuyh23chjwemve7p9t7sl63v0sxtjh84e95w4fdz3htg8gmgspua7q4
[23]: https://osmosis.celat.one/osmosis-1/contracts/osmo1aye5qcer5n52crrkaf35jprsad2807q6kg3eeeu7k79h4slxfausfqhc9y
[24]: https://neutron.celat.one/neutron-1/contracts/neutron17yehp4x7n79zq9dlw4g7xmnrvwdjjj2yecq26844sg8yu74knlxqfx5vqv
[25]: https://neutron.celat.one/neutron-1/contracts/neutron1aszpdh35zsaz0yj80mz7f5dtl9zq5jfl8hgm094y0j0vsychfekqxhzd39
[26]: https://neutron.celat.one/neutron-1/contracts/neutron1dwp6m7pdrz6rnhdyrx5ha0acsduydqcpzkylvfgspsz60pj2agxqaqrr7g
[27]: https://neutron.celat.one/neutron-1/contracts/neutron1n97wnm7q6d2hrcna3rqlnyqw2we6k0l8uqvmyqq6gsml92epdu7quugyph
[28]: https://neutron.celat.one/neutron-1/contracts/neutron1h4l6rvylzcuxwdw3gzkkdzfjdxf4mv2ypfdgvnvag0dtz6x07gps6fl2vm
[29]: https://neutron.celat.one/neutron-1/contracts/neutron1udr9fc3kd743dezrj38v2ac74pxxr6qsx4xt4nfpcfczgw52rvyqyjp5au
[36]: https://www.mintscan.io/mars-protocol/accounts/mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x
[37]: https://www.mintscan.io/mars-protocol/accounts/mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575
[38]: https://go.dev/
[39]: https://github.com/osmosis-labs/test-tube
[40]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.0
[41]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.1
[42]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.2
[43]: https://github.com/mars-protocol/rover/releases/tag/v2.0.0
[44]: https://github.com/mars-protocol/contracts/releases/tag/v1.2.0
[45]: https://github.com/mars-protocol/contracts/releases/tag/v1.2.1
[46]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.3
[47]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.4
[48]: https://github.com/mars-protocol/contracts/releases/tag/v1.2.2
[49]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.5
[50]: https://github.com/mars-protocol/contracts/releases/tag/v2.0.7
[51]: https://github.com/mars-protocol/contracts/releases/tag/v1.3.0
[52]: https://neutron.celat.one/neutron-1/contracts/neutron184kvu96rqtetmunkkmhu5hru8yaqg7qfhd8ldu5avjnamdqu69squrh3f5
[53]: https://neutron.celat.one/neutron-1/contracts/neutron1qdzn3l4kn7gsjna2tfpg3g3mwd6kunx4p50lfya59k02846xas6qslgs3r
[54]: https://neutron.celat.one/neutron-1/contracts/neutron17ktfwsr7ghlxzzma0gw0hke3j3rnssd58q87jv2wzfrk6uhawa3sv8xxtm
[55]: https://neutron.celat.one/neutron-1/contracts/neutron1x4rgd7ry23v2n49y7xdzje0743c5tgrnqrqsvwyya2h6m48tz4jqqex06x
[56]: https://neutron.celat.one/neutron-1/contracts/neutron1dr0ckm3u2ztjuscmgqjr85lwyduphxkgl3tc02ac8zp54r05t5dqp5tgyq
[57]: https://github.com/mars-protocol/contracts/releases/tag/v2.1.0