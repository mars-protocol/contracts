# Mars Swapper

`mars-swapper-base` contains chain-agnostic logics for the swapper contract. Each chain specific implementation should implement its own route struct that should implement the `mars_swapper_base::Route` trait and then use the `mars_swapper_base::SwapBase` to implement the entry point functions. See `./osmosis/src/contract.rs` for an example. Each chain specific swapper will thus implement the same API.

The swapper contracts should NEVER hold any funds and any funds sent to the contract except as part of executing the `SwapExactIn` message can be stolen by an attacker. See [Oak Audit 2023-08-01](https://github.com/oak-security/audit-reports/blob/master/Mars/2023-08-01%20Audit%20Report%20-%20Mars%20Red%20Bank%20Updates%20v1.0.pdf) issue 14.

## Mars Swapper Mock

Mock swapper contains a mock swapper contract to be used for testing purposes only. It only implements `ExecuteMsg::SwapExactIn` and `QueryMsg::EstimateExactInSwap`. When calling `ExecuteMsg::SwapExactIn` `denom_out` must be `uosmo` and the resulting amount will always be `1337uosmo`. The contract MUST be prefunded with this amount.
