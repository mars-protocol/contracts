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
    addressProviderSet: Record<string, boolean>
    proposedNewOwner?: boolean
    acceptedOwnership?: boolean
    seedMockVault?: boolean
    grantedCreditLines?: boolean
    redBankMarketsSet: string[]
    assetsSet: string[]
    vaultsSet: string[]
    oraclePricesSet: string[]
    routesSet: string[]
    healthContractConfigUpdate?: boolean
    creditManagerContractConfigUpdate?: boolean
  }
}
