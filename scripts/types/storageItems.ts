export interface StorageItems {
  codeIds: {
    accountNft?: number
    mockRedBank?: number
    mockVault?: number
    mockOracle?: number
    swapper?: number
    creditManager?: number
  }
  addresses: {
    accountNft?: string
    mockRedBank?: string
    mockVault?: string
    mockOracle?: string
    swapper?: string
    creditManager?: string
  }
  actions: {
    proposedNewOwner?: boolean
    acceptedOwnership?: boolean
  }
}
