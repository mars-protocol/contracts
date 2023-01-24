# Mars Oracle - Base

Chain-agnostic logics for the oracle contract. To create an oracle contract specific to a chain, create a `{chain-name}PriceSource` object that implements the `PriceSource` trait, which defines methods relevant for acquiring price data on that chain; then, plug it in to the `OracleBase` type.

Taking the [Osmosis](https://github.com/osmosis-labs/osmosis) chain for example:

```rust
use mars_oracle_base::{OracleBase, PriceSource};
use osmo_bindings::OsmosisQuery;

enum OsmosisPriceSource {
  // ...
}

impl PriceSource<OsmosisQuery> for OsmosisPriceSource {
  // ...
}

type OsmosisOracle<'a> = OracleBase<'a, OsmosisQuery, OsmosisPriceSource>;
```

## License

Contents of this crate are open source under [GNU General Public License v3](../../../LICENSE) or later.
