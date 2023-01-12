# Rover
A generalized credit protocol built on Mars lending market

## Overview

DeFi lending protocols, such as Aave and Compound, typically require users to first deposit some collateral assets before they can borrow. Once deposited, this collateral is locked inside the lending market smart contracts; the users are not allowed to put them into productive use as they see fit.

Rover takes a different approach for Mars protocol, which utilizes a generalized credit account manager built on top of Mars' Red Bank. This approach will allow borrowers to retain control of their collateral assets, while at the same time providing a return for lenders.

### Credit manager and credit accounts


The target audience of the credit manager is risk-seeking investors who wish to undertake leveraged trading or yield farming activities.

To start, a user first needs to access the Mars credit manager contract and request the opening of a credit account. The credit account is analogous to a "sub-account" on centralized trading platforms such as FTX, and is represented by a non-fungible token (NFT).

```rust
pub enum ExecuteMsg {
    /// Create a new account
    CreateCreditAccount {},
    /// Take actions on account
    UpdateCreditAccount {
        account_id: String,
        actions: Vec<Action>,
    },
}
```

Users interact with their credit accounts by executing the following actions:

```rust
pub enum Action {
    Deposit(Coin),
    Withdraw(Coin),
    Borrow(Coin),
    Repay(Coin),
    EnterVault {
        vault: VaultUnchecked,
        denom: String,
        amount: Option<Uint128>,
    },
    ExitVault {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    RequestVaultUnlock {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    ExitVaultUnlocked { id: u64, vault: VaultUnchecked },
    LiquidateCoin {
        liquidatee_account_id: String,
        debt_coin: Coin,
        request_coin_denom: String,
    },
    LiquidateVault {
        liquidatee_account_id: String,
        debt_coin: Coin,
        request_vault: VaultUnchecked,
    },
    SwapExactIn {
        coin_in: Coin,
        denom_out: String,
        slippage: Decimal,
    },
    ProvideLiquidity {
        coins_in: Vec<Coin>,
        lp_token_out: String,
        minimum_receive: Uint128,
    },
    WithdrawLiquidity { lp_token: Coin },
    RefundAllCoinBalances {},
}
```

The credit manager contract executes the list of actions specified by `ExecuteMsg::UpdateCreditAccount { actions }` in order. *After all actions have been executed, it calculates the overall health factor of the credit account. If the account is unhealthy as a result of the actions, an error is thrown and all actions reverted.*

You may have noticed that this design resembles [the Fields of Mars contract](https://github.com/mars-protocol/fields-of-mars/blob/v1.0.0/packages/fields-of-mars/src/martian_field.rs#L264-L318) in Mars V1. Indeed, Credit Manager can be considered a direct extension of Fields.

### Vault API

Vault writers interested in integrating with Rover must write their vaults
to abide by the [Cosmos Vault Standard](https://github.com/apollodao/cosmos-vault-standard) and
make a governance proposal.

### Additional thoughts

A generalized credit protocol would enable leveraged trading or yield farming capabilities that are largely available only on centralized exchanges today. End users would be able to borrow more than they've deposited into the lending protocol while still ensuring all credit accounts are fully collateralized. Mars Hub's decentralized architecture would also give third parties the ability to propose and write new trading and yield farming strategies that could tap Red Bank deposits on any supported blockchain. The proposed protocol would increase utility for traders. This new source of demand may generate higher yields for depositors and more fees for Mars Hub and MARS stakers.

## Development

### Environment Setup

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

- Also note that Intel/Amd 64-bit processor is required. While there is experimental ARM support for CosmWasm/rust-optimizer, it's discouraged to use in production and LocalOsmosis is likely to have issues.

### Build

Pull down Rover repo locally
```shell
git clone https://github.com/mars-protocol/rover
cd rover
```

Run `cargo build` to ensure it compiles fine.


### Test

Requires building the wasm binaries for the contracts:
```shell
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.10
```

For Rust cw-multi tests + osmosis-testing suite (requires mars_swapper_osmosis.wasm from previous step):
```shell
cargo test
```

For Typescript end-to-end testnet deployment & tests against that deployment:
```shell
cd scripts
yarn install
yarn deploy:osmosis
```

### Deployment

Addresses published in [/scripts/deploy/addresses](https://github.com/mars-protocol/rover/tree/master/scripts/deploy/addresses)


## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.
