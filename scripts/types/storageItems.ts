export interface StorageItems {
  codeIds: {
    accountNft?: number
    mockRedBank?: number
    mockVault?: number
    mockOracle?: number
    swapper?: number
    zapper?: number
    healthContract?: number
    creditManager?: number
  }
  addresses: {
    accountNft?: string
    mockVault?: string
    swapper?: string
    zapper?: string
    healthContract?: string
    creditManager?: string
    params?: string
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
  }
  owner?: string
}
