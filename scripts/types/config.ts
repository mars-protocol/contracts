export interface DeploymentConfig {
  oracleAddr: string
  redBankAddr: string
  baseDenom: string
  secondaryDenom: string
  chainPrefix: string
  rpcEndpoint: string
  deployerMnemonic: string
  vaultTokenDenom: string
  chainId: string
  defaultGasPrice: number
  startingAmountForTestUser: number
  depositAmount: number
  borrowAmount: number
  repayAmount: number
  swapAmount: number
  slippage: number
  swapRoute: { steps: { denom_out: string; pool_id: number }[] }
  withdrawAmount: number
  maxCloseFactor: number
  maxLiquidationBonus: number
}
