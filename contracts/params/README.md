# Mars Params Contract

The Mars Params Contract is published to [Crates.io](https://crates.io/crates/mars-params)

This contract holds the following values for all the assets in Mars Protocol: 

- **Max Loan To Value:** Max percentage of collateral that can be borrowed
- **Liquidation Threshold:** LTV at which the loan is defined as under collateralized and can be liquidated
- **Liquidation Bonus:** Percentage of extra collateral the liquidator gets as a bonus
- **Deposit Enabled:** Is the asset able to be deposited into the Red Bank
- **Borrow Enabled:** Is the asset able to be borrowed from the Red Bank
- **Deposit Cap:** Max amount that can be deposited into the Red Bank
- **Asset Settings:** Credit Manager and Red Bank Permission Settings

Note: Credit Manager Vaults only utilize max loan to value, liquidation threshold, and deposit cap parameters, while Red Bank Markets utilize all of the above parameters.

## High Levered Strategies (HLS)

An HLS is a position where the borrowed asset is highly correlated to the collateral asset (e.g. atom debt -> stAtom collateral). 
This has a low risk of liquidation. For this reason, Credit Manager grants higher MaxLTV & LiqThreshold parameters,
granting higher leverage. An asset's HLS parameters are stored in this contract and are applied to credit accounts 
of the HLS type during a health check.

### De-listing an HLS asset

There are a few scenarios depending on what denom is being de-listed. Always communicate each step to the users!
- **De-listing a collateral denom**: 
  - Set the MaxLTV of the denom to zero. 
  - Gradually reduce the HLS Liquidation Threshold to zero.
  - _Do not_ set HLS parameters to None or remove it from correlations list for debt denom. This would result in freezing the HLS accounts that have that collateral.
- **De-listing a debt denom**:
  - Set the MaxLTV of all denoms in the debt denom's correlations list to zero. 
  - Gradually reduce the HLS Liquidation Threshold to zero.
  - _Do not_ set HLS parameters to None. This would result in freezing the HLS accounts that have that debt denom.
