export interface StorageItems {
  codeIds: {
    redBank?: number
    rewardsCollector?: number
    addressProvider?: number
    incentives?: number
    oracle?: number
  }
  addresses: {
    addressProvider?: string
    rewardsCollector?: string
    redBank?: string
    incentives?: string
    oracle?: string
  }

  execute: {
    addressProviderUpdated?: boolean
    assetsInitialized: string[]
    secondAssetInitialized?: boolean
    oraclePriceSet?: boolean
    smokeTest?: boolean
  }
}
