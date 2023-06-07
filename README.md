# Mars Protocol: Common Contracts

## Params Contract 
This contract holds the following values for all the assets in Mars Protocol.
- **Reserve Factor:** Percentage of borrowed interest that stays in the protocol as fees
- **Max Loan To Value:** Max percentage of collateral that can be borrowed
- **Liquidation Threshold:** LTV at which the loan is defined as under collateralized and can be liquidated
- **Liquidation Bonus:** Percentage of extra collateral the liquidator gets as a bonus
- **Deposit Enabled:** Is the asset able to be deposited into the Red Bank 
- **Borrow Enabled:** Is the asset able to be borrowed from the Red Bank
- **Target Utilization Rate:** 
  - Base: Interest Rate at 0 utilization rate
  - slope 1: Slope for when U < Uoptimal
  - slope 2: Slope for when U > Uoptimal
- **Deposit Cap:** Max amount that can be deposited into the Red Bank

Note: Credit Manager Vaults only utilize max loan to value, liquidation threshold, and deposit cap parameters, while Red Bank Markets utilize all of the above parameters. 