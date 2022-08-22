# Liquidation filter
The liquidation-filter contract queries the health status of each account to be liquidated on the current block height, and if the account is no longer health factor < 1 it will remove that liquidation from the msg in order to let other liquidation messages go through successfully.
