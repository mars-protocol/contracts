export interface StorageItems {
  codeIds: {
    accountNft?: number
    mockRedBank?: number
    mockVault?: number
    mockOracle?: number
    marsOracleAdapter?: number
    swapper?: number
    creditManager?: number
  }
  addresses: {
    accountNft?: string
    mockVault?: string
    marsOracleAdapter?: string
    swapper?: string
    creditManager?: string
  }
  actions: {
    proposedNewOwner?: boolean
    acceptedOwnership?: boolean
    setRouteAndSeedSwapper?: boolean
    seedMockVault?: boolean
  }
}
