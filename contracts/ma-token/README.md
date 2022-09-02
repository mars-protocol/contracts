# maToken

maToken is a modified cw20 that is minted in representation of a deposited asset.
Each deposited asset has a corresponding instance of the maToken and accumulate interest in the way that they are redeemable for an ever increasing amount of their underlying asset.
The Red Bank can do forced transfers/burns when user positions are being liquidated.
On each contract call that changes a balance, the maToken will call the incentives contract in order to manage MARS rewards.
