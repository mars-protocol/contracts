import { CoinMarketInfo } from './generated/mock-red-bank/MockRedBank.types'
import { Coin } from '@cosmjs/amino'
import { CoinPrice } from './generated/mock-oracle/MockOracle.types'

export interface DeploymentConfig {
  baseDenom: string
  secondaryDenom: string
  chainPrefix: string
  rpcEndpoint: string
  deployerMnemonic: string
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
  mockRedbankCoins: CoinMarketInfo[]
  seededFundsForMockRedBank: Coin[]
  oraclePrices: CoinPrice[]
  maxCloseFactor: number
  maxLiquidationBonus: number
}
