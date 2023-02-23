import { Duration, VaultInfoResponse } from './generated/mars-mock-vault/MarsMockVault.types'
import {
  VaultConfig,
  VaultInstantiateConfig,
} from './generated/mars-credit-manager/MarsCreditManager.types'
import { PriceSource } from './priceSource'

export enum VaultType {
  LOCKED,
  UNLOCKED,
}

export interface VaultInfo {
  lockup: { time: number } | undefined
  tokens: VaultInfoResponse
}

export interface DeploymentConfig {
  chain: {
    prefix: string
    id: string
    rpcEndpoint: string
    defaultGasPrice: number
    baseDenom: string
  }
  deployerMnemonic: string
  oracle: { addr: string }
  redBank: { addr: string }
  vaults: VaultInstantiateConfig[]
  allowedCoins: string[]
  maxCloseFactor: string
  maxValueForBurn: string
  maxUnlockingPositions: string
  swapRoutes: SwapRoute[]
  testActions?: TestActions
  swapperContractName: string
  zapperContractName: string
  multisigAddr?: string
}

export interface SwapRoute {
  denomIn: string
  denomOut: string
  route: { token_out_denom: string; pool_id: string }[]
}

export interface TestActions {
  allowedCoinsConfig: { denom: string; priceSource: PriceSource; grantCreditLine: boolean }[]
  vault: {
    depositAmount: string
    withdrawAmount: string
    mock: {
      type: VaultType
      config: VaultConfig
      vaultTokenDenom: string
      lockup?: Duration
      baseToken: string
    }
  }
  outpostsDeployerMnemonic: string
  secondaryDenom: string
  defaultCreditLine: string
  startingAmountForTestUser: string
  depositAmount: string
  lendAmount: string
  borrowAmount: string
  repayAmount: string
  swap: {
    amount: string
    slippage: string
    route: { token_out_denom: string; pool_id: string }[]
  }
  withdrawAmount: string
  zap: {
    coinsIn: { amount: string; denom: string }[]
    denomOut: string
  }
  unzapAmount: string
}
