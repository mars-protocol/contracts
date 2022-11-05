import { Coin } from './generated/credit-manager/CreditManager.types'

export enum VaultType {
  LOCKED,
  UNLOCKED,
}

export interface DeploymentConfig {
  oracleAddr: string
  redBankAddr: string
  baseDenom: string
  secondaryDenom: string
  chainPrefix: string
  rpcEndpoint: string
  deployerMnemonic: string
  redBankDeployerMnemonic: string
  vaultTokenDenom: string
  chainId: string
  defaultGasPrice: number
  startingAmountForTestUser: number
  depositAmount: number
  toGrantCreditLines: Coin[]
  borrowAmount: number
  repayAmount: number
  swapAmount: number
  slippage: number
  swapRoute: { steps: { denom_out: string; pool_id: number }[] }
  withdrawAmount: number
  maxCloseFactor: number
  maxLiquidationBonus: number
  vaultType: VaultType
  vaultDepositAmount: number
  vaultDepositCap: Coin
  vaultLiquidationThreshold: number
  vaultMaxLTV: number
  vaultWithdrawAmount: number
  lpToken: { denom: string; price: number }
  zap: { amount: number; denom: string; price: number }[]
  unzapAmount: number
}
