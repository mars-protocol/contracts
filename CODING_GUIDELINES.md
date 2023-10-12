# Coding Guidelines

This document provides details about the coding guidelines and requirements.

## Project structure

```
contracts
├── incentives
│   ├── src
│   │   ├── lib.rs
│   │   ├── contract.rs
│   │   ├── error.rs
│   │   ├── helpers.rs
│   │   └── state.rs
│   └── tests
│       ├── helpers.rs
│       └── test_incentives.rs
├── oracle
│   ├── base
│   └── osmosis
└── red-bank
packages
├── types
└── testing
..
```

- All contracts should be placed in `contracts` directory. Chain specific contracts should have `base` directory for common code and chain (for example `osmosis`) directory for its implementation.
- Contract messages (Instantiate, Execute, Query etc.) and common helpers should be in `packages`.

### Single contract

- Place instantiate, execute and query logic in separate files (if single contract file is too big):
  - `contract.rs` which contains the entry points,
  - `execute.rs` which contains the execute functions,
  - `query.rs` which contains the query functions.
- Use things in `execute` / `query` with module prefix for example: `execute::deposit()`, `query::query_market()`.
- Place all unit tests in separate directory "tests" (see how modules/imports work for this directory https://doc.rust-lang.org/book/ch11-03-test-organization.html).
  One test file prefixed with `test_` (test_ONE_OF_EXECUTE_MSG.rs) should contain all test cases for single executed message.

### Packages

To simplify importing module things from packages, for example:

```
packages
└── outpost
    └── src
        └── red-bank
            ├── mod.rs
            ├── thing_one.rs
            ├── thing_two.rs
            └── thing_three.rs
...
```

reimport the sub-modules in `mod.rs` file:

```rust
mod thing_one;
mod thing_two;
mod thing_three;

pub use thing_one::*;
pub use thing_two::*;
pub use thing_three::*;
```

Later we can import `red-bank` module things with one line:

```rust
use mars_types::red_bank::*;
```

## API & Design

### Naming

- Variables of `Addr` type should be named `something_addr`; it unchecked variant (of `String` type) should be named `something` (without the `*_addr` suffix).
- Query messages related to a single asset should be named `QueryMsg::Something` (singular), while their corresponding enumerative queries for all assets should be named `QueryMsg::Somethings` (plural).

### Attributes for indexing

```
key: "action", value: EXECUTE_MSG
```

- Everything should be snake case (e.g. user_address).

```rust
Response::new()
    .add_attribute("action", "balance_change")
    .add_attribute("balance_scaled", "100")
    .add_attribute("user", "user_address")
```

### Panics (out of gas)

- Avoid unwraps.
- Use the `?` operator and `try_*` methods (`try_for_each`, `try_fold` etc.) for handling errors.
- Use checked arithmetic (to avoid overflows, division by zero).

### Errors

Use https://crates.io/crates/thiserror to provide enum with contract errors, for example:

```rust
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Asset is not enabled for distribution: {denom}")]
    AssetNotEnabledForDistribution {
        denom: String,
    },

    #[error("Amount to distribute {amount} is larger than available balance {balance}")]
    AmountToDistributeTooLarge {
        amount: Uint128,
        balance: Uint128,
    },

    #[error("Invalid route: {reason}")]
    InvalidRoute {
        reason: String,
    },
}
```

### Schema

Generate schema files (CI/CD fails on outdated schema):

```
cargo make generate-all-schemas
```

### Zero comparisons

To compare a `Uint128` or `Decimal` to zero, use the built-in `is_zero` method:

```rust
let a = cosmwasm_std::Uint128::new(some_number);
let b = cosmwasm_std::Decimal::percent(some_number);

// NOT recommended
if a == Uint128::zero() {
  // ...
}
if b > Decimal::zero() {
  // ...
}

// recommended
if a.is_zero() {
  // ...
}
if !b.is_zero() {
  // ...
}
```

## Modularization

In case of chain specific logic, make the whole contract a portable object with a generic:

```rust
trait Adapter {
    // define functions the base contract use
}

struct BaseContract<Adapter> {
    // define common functionality for all chains
}
```

The `BaseContract` struct will contain logics that are common to all chains.

Then for each chain, we create their adapter type:

```rust
struct OsmosisAdapter {
    // ...
}

impl Adapter for OsmosisAdapter {
    // ...
}
```

The `OsmosisAdapter` struct will contain logics specific to osmosis, e.g. how to use osmosis dex to swap tokens.

Similarly we can create `InjectiveAdapter`, `SeiAdapter`, etc.

Finally, in order to create the contract for a specific chain, we simply plug the adapter into the contract:

```rust
type ContractForOsmosis = BaseContract<OsmosisAdapter>;
```

## Testing

### Test helpers

Each file in the `tests` folder is [treated as an individual crate](https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-directory). As a result of this, if a file only contains helper functions to be used by other files, and does not contain any tests that use these functions in itself, the Rust compiler will raise "function defined but not used" errors. Consider adding the following line at the top of the file to suppress this warning:

```rust
#![allow(dead_code)]
```

### Doc tests

If a crate does not contain documentations to be tested, considering adding the following configuration in `Cargo.toml` to disable doc-tests:

```toml
[lib]
doctest = false
```

### Code coverage

One of the basic metrics over code quality is how much is covered by unit tests.
CI/CD fails on decrease of the code coverage.

To check code coverage locally:

```
cargo make coverage-grcov-html
```

or:

```
cargo make coverage-grcov-lcov
```

The report can be found in `target/coverage` directory (for example: `target/coverage/html/index.html`).

## CI/CD

Setting up a pipeline with strict checks helps ensure only linted+tested code merged.

- Setup a task runner. _Cargo make_ is recommended. Here’s an example: [https://github.com/mars-protocol/rover/blob/master/Makefile.toml](https://github.com/mars-protocol/rover/blob/master/Makefile.toml). Tasks to test for:
  - Building
  - Linting
  - Formatting
  - Testing
  - Generate latest schemas
  - Contract compilation
- Setup Github workflow that runs all checks when pull requests are open: [https://github.com/mars-protocol/rover/blob/master/.github/workflows](https://github.com/mars-protocol/rover/blob/master/.github/workflows).
- Ensure the master branch has protections to not allow merges without passing checks.
