# Rover
A generalized credit protocol built on Mars lending market

## Bug bounty

## Overview

DeFi lending protocols, such as Aave and Compound, typically require users to first deposit some collateral assets before they can borrow. Once deposited, this collateral is locked inside the lending market smart contracts; the users are not allowed to put them into productive use as they see fit.

Rover takes a different approach for Mars protocol, which utilizes a generalized credit account manager built on top of Mars' Red Bank. This approach will allow borrowers to retain control of their collateral assets, while at the same time providing a return for lenders.

### Credit manager and credit accounts


The target audience of the credit manager is risk-seeking investors who wish to undertake leveraged trading or yield farming activities.

To start, a user first needs to access the Mars credit manager contract and request the opening of a credit account. The credit account is analogous to a "sub-account" on centralized trading platforms such as FTX, and is represented by a non-fungible token (NFT).

Users interact with their credit accounts by executing actions. In Rust/[CosmWasm](https://cosmwasm.com/) code, this can be expressed as (using the [cw-asset](https://github.com/mars-protocol/cw-asset) library):

```rust
use cosmwasm_std::Uint128;
use cw_asset::{Asset, AssetList, AssetInfo};

enum Action {
    /// deposit the specified asset into the credit account
    Deposit(Asset),
    /// withdraw the specified asset from the credit account
    Withdraw(Asset),
    /// borrow the specified asset from Red Bank
    Borrow(Asset),
    /// repay the specified asset to Red Bank
    Repay(Asset),
    /// swap the asset in the credit account
    Swap {
        offer: Asset,
        ask: AssetInfo,
        minimum_receive: Option<Uint128>,
    },
    /// deposit assets into a vault (e.g. an automated
    /// yield farming strategy)
    EnterVault {
        vault_addr: String,
        deposits: AssetList,
    },
    /// withdraw assets from a vault
    ExitVault {
        vault_addr: String,
        shares: Uint128,
    },
}

enum ExecuteMsg {
    /// mint a new credit account NFT
    CreateCreditAccount {
        initial_deposits: AssetList,
    },
    /// update a credit account specified by the token id
    /// by executing an array of actions
    UpdateCreditAccount {
        token_id: String,
        actions: Vec<Action>,
    },
}

```

The credit manager contract executes the list of actions specified by `ExecuteMsg::UpdateCreditAccount { actions }` in order. *After all actions have been executed, it calculates the overall health factor of the credit account. If the account is unhealthy as a result of the actions, an error is thrown and all actions reverted.*

Some readers might have noticed that this design resembles [the Fields of Mars contract](https://github.com/mars-protocol/fields-of-mars/blob/v1.0.0/packages/fields-of-mars/src/martian_field.rs#L264-L318) in Mars V1. Indeed, the credit account can be considered a direct extension of Fields, of which many code components can be reused.

### Deposit and borrowing


In the example below, the user funds a freshly-opened credit account with 50 USDC, and borrows 100 USDC from Mars liquidity pool. To do this, provide the following execute message:

```json
{
  "update_credit_account": {
    "token_id": "...",
    "actions": [
      {
        "deposit": {
          "info": {
            "native": "uusdc"
          },
          "amount": "50000000"
        }
      },
      {
        "borrow": {
          "info": {
            "native": "uusdc"
          },
          "amount": "100000000"
        }
      }
    ]
 }
}

```

The actions results in the following credit account position:

[![Screen Shot 2022-06-24 at 12.43.09 AM](https://aws1.discourse-cdn.com/standard17/uploads/mars/optimized/1X/c12fca3fae645092b9deba67c9398fe0795b0c38_2_690x280.png)

This example highlights a few characteristics of the credit account:

1.  The credit manager does not need to explicitly deposit collateral into Red Bank before borrowing from it. In other words, the loan is extended to the credit manager in the form of uncollateralized debt. (*NOTE: Martian Council must have approved an uncollateralized limit*)
2.  The user borrows more assets ($100) than their deposit ($50). To our knowledge, this is not possible with other lending protocols. Despite this, the credit account remains over-collateralized ($150 in assets vs $100 in liabilities).
3.  A production-ready credit manager contract will probably need to use a more sophisticated algorithm to assess the health of credit accounts, which takes into consideration various risk factors such as the volatility and market liquidity of each supported asset. In this article, we use LTV for simplicity sake.

### Leveraged trading

Credit accounts support trading of assets. Continuing from the previous example, the user may provide the following execute message, which swaps 50 USDC for OSMO (assuming OSMO price is ~$2):

```json
{
  "update_credit_account": {
    "token_id": "...",
    "actions": [
      {
        "swap": {
          "offer": {
            "info": {
              "native": "uusdc"
            },
            "amount": "50000000"
          },
          "ask": {
            "native": "uosmo"
          },
          "minimum_receive": "25000000"
        }
      }
    ]
  }
}

```

Which results in the following credit account position:

[![Screen Shot 2022-06-24 at 12.43.17 AM](https://aws1.discourse-cdn.com/standard17/uploads/mars/optimized/1X/fee50699046039aa809df51489d5c0d4b5000636_2_690x285.png)

Although the trade takes place on a spot exchange, since it is (partially) funded by borrowed assets from Red Bank, the user here effectively takes a leveraged long position on OSMO.

### Vaults

The credit manager may support vaults created by third party protocols, which provide automated trading or yield farming strategies, as long as these vaults are approved by Martian Council (governance), and implement a standard API.

The API includes execute functions for entering or exiting, and query functions for assessing the asset value locked in the vault. The execute functions also must emit events in a specific format (out of scope for this article) so that they can be parsed by the credit manager.

```rust
/// each vault contract must implement this
enum VaultExecuteMsg {
    Enter {
        deposits: AssetList,
    },
    Exit {
        shares: Uint128,
    },
}

/// each vault contract must implement this
enum VaultQueryMsg {
    /// the amount of assets under management of the vault that
    /// belongs to a specific user; response type: `cw_asset::AssetList`
    Deposit {
        user: String,
    },
}

```

For example, assume a vault that takes OSMO and USDC deposits, provides them to an Osmosis DEX pool, stakes the LP share tokens, and auto-compounds staking rewards. To enter this vault, the user executes:

```json
{
  "update_credit_account": {
    "token_id": "...",
    "actions": [
      {
        "enter_vault": {
          "vault_addr": "...",
          "deposits": [
            {
              "info": {
                "native": "uosmo"
              },
              "amount": "25000000"
            },
            {
              "info": {
                "native": "uusdc"
              },
              "amount": "50000000"
            }
          ]
        }
      }
    ]
  }
}

```

Which results in:

[![Screen Shot 2022-06-24 at 12.43.24 AM](https://aws1.discourse-cdn.com/standard17/uploads/mars/optimized/1X/30e208b40a0812560bb2606260efb94035517f42_2_690x286.png)

*NOTE: It is possible to execute the two previous steps (swap and enter vault) in one transaction, improving user experience.*

### Liquidations

To ensure solvency (i.e. that the protocol always has more assets than liabilities), the credit manager must monitor the health factor of each credit account, and execute liquidations when necessary.

A credit account's health factor may drop due to:

-   the value of assets going down (e.g. prices dropping)
-   the value of liabilities going up (e.g. borrow interest accrues, or prices of the debt assets going up)

Once the health factor drops below a preset threshold (liquidation threshold, a governance-decided parameter), any person can trigger a liquidation of the credit account. The liquidator must pay back some amounts of debts on behalf of the credit account; they will in return be rewarded some of the account's assets as the liquidation bonus:

[![Screen Shot 2022-06-24 at 12.46.36 AM](https://aws1.discourse-cdn.com/standard17/uploads/mars/optimized/1X/a5c0da751deceb28652bbd82127c41ee2c01d507_2_493x500.png)

The liquidator may then sell the vault shares in the secondary market.

In this example, the user account's net value falls from $20 ($120 in assets minus $100 in liabilities) to $15 (losing $5), while the liquidator's account net value increases by $5. The transfer of this $5 is the liquidation bonus, rewarded to the liquidator for deploying capital to ensure the protocol's solvency. The maximum allowed amount of bonus for each liquidation event, the bonus rate, can either be set by governance (i.e. the approach used by Aave) or by [free market mechanisms 2](https://twitter.com/larry0x/status/1538515908049747971) ([Euler](https://twitter.com/euler_mab/status/1537091423748517889)).

### Additional thoughts

A generalized credit protocol would enable leveraged trading or yield farming capabilities that are largely available only on centralized exchanges today. End users would be able to borrow more than they've deposited into the lending protocol while still ensuring all credit accounts are fully collateralized. Mars Hub's decentralized architecture would also give third parties the ability to propose and write new trading and yield farming strategies that could tap Red Bank deposits on any supported blockchain. The proposed protocol would increase utility for traders. This new source of demand may generate higher yields for depositors and more fees for Mars Hub and MARS stakers.

## Development

### Environment Setup

#### ==== For building/testing contracts in Rust ===

[Install rustup](https://rustup.rs/). Once installed, make sure you have the wasm32 target:
```shell
rustup default stable
cargo version
# If this is lower than 1.55.0+, update
rustup update stable
rustup target list --installed
rustup target add wasm32-unknown-unknown
```

Run `cargo build` and you're good to go!

#### ==== For end-to-end tests and deployment scripts ===

 - Dependencies
     - [Docker](https://docs.docker.com/get-docker/)
     - [Node.js v16](https://github.com/nvm-sh/nvm)
     - [LocalOsmosis](https://docs.osmosis.zone/developing/dapps/get_started/cosmwasm-localosmosis.html#initial-setup)
     - Intel/Amd 64-bit processor
         - while there is experimental Arm support for CosmWasm/rust-optimizer, it's discouraged to use in production and LocalOsmosis is likely to have issues.

### Test

For contract tests in rust
```shell
cargo test
```

For end-to-end tests via Typescript. Start LocalOsmosis:
```shell
cd LocalOsmosis
make start
```

In another shell, compile contracts

```shell
# In rover directory at the top level
docker run --rm -v "$(pwd)":/code \
--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
cosmwasm/workspace-optimizer:0.12.6
```

Run test scripts

```shell
cd scripts
npm install
npm run test
```

### Deploy

### Notes

## Deployment

### Mainnet

### Testnet

## License
