export interface StorageItems {
  codeIds: {
    'red-bank'?: number
    'rewards-collector'?: number
    'address-provider'?: number
    incentives?: number
    oracle?: number
  }
  addresses: {
    'address-provider'?: string
    'rewards-collector'?: string
    'red-bank'?: string
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

  owner?: string
}
