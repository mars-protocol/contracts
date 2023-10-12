export interface StorageItems {
  codeIds: {
    accountNft?: number
    addressProvider?: number
    creditManager?: number
    health?: number
    incentives?: number
    mockVault?: number
    oracle?: number
    params?: number
    swapper?: number
    redBank?: number
    rewardsCollector?: number
    zapper?: number
  }
  addresses: {
    accountNft?: string
    addressProvider?: string
    creditManager?: string
    health?: string
    incentives?: string
    mockVault?: string
    oracle?: string
    params?: string
    swapper?: string
    redBank?: string
    rewardsCollector?: string
    zapper?: string
  }
  actions: {
    proposedNewOwner?: boolean
    acceptedOwnership?: boolean
    setRoutes?: boolean
    seedMockVault?: boolean
    grantedCreditLines?: boolean
    oraclePricesSet?: boolean
    redBankMarketsSet?: boolean
    healthContractConfigUpdate?: boolean
    creditManagerContractConfigUpdate?: boolean
  }
  execute: {
    addressProviderUpdated: Record<string, boolean>
    assetsUpdated: string[]
    marketsUpdated: string[]
    vaultsUpdated: string[]
    oraclePriceSet?: boolean
    smokeTest?: boolean
  }
  owner?: string
}
