# Mars Params Contract

The Mars Params Contract is published to [Crates.io](https://crates.io/crates/mars-params)

This contract holds the following values for all the assets in Mars Protocol: 

- **Max Loan To Value:** Max percentage of collateral that can be borrowed
- **Liquidation Threshold:** LTV at which the loan is defined as under collateralized and can be liquidated
- **Liquidation Bonus:** Percentage of extra collateral the liquidator gets as a bonus
- **Deposit Enabled:** Is the asset able to be deposited into the Red Bank
- **Borrow Enabled:** Is the asset able to be borrowed from the Red Bank
- **Deposit Cap:** Max amount that can be deposited into the Red Bank
- **Asset Permissions** Rover and Red Bank Permission Settings

Note: Rover Vaults only utilize max loan to value, liquidation threshold, and deposit cap parameters, while Red Bank Markets utilize all of the above parameters.

