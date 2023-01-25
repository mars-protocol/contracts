# Mars Rewards Collector - Base

Chain-agnostic logics for the rewards collector contract. To create an oracle contract specific to a chain, create a `{chain-name}Route` object that implements and `Route` trait, which defines methods relevant for swapping assets on that chain; then plugin it into the `CollectorBase` type.

Taking the [Osmosis](https://github.com/osmosis-labs/osmosis) chain for example:

```rust
use mars_rewards_collector_base::{CollectorBase, Route};
use osmo_bindings::{OsmosisMsg, OsmosisQuery, Step};

// the route is an array of `Step`s
struct OsmosisRoute(pub Vec<Step>);

impl Route<OsmosisMsg, OsmosisQuery> for OsmosisRoute {
  // ...
}

pub type OsmosisCollector<'a> = CollectorBase<'a, OsmosisRoute, OsmosisMsg, OsmosisQuery>;
```

## License

Contents of this crate are open source under [GNU General Public License v3](../../../LICENSE) or later.
