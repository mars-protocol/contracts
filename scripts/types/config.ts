import { Duration, VaultInfoResponse } from './generated/mars-mock-vault/MarsMockVault.types'
import { PriceSource } from './priceSource'
import { VaultConfigBaseForString } from './generated/mars-params/MarsParams.types'

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
  params: { addr: string }
  vaults: VaultConfigBaseForString[]
  allowedCoins: string[]
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
      config: Omit<VaultConfigBaseForString, 'addr'>
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
  reclaimAmount: string
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
