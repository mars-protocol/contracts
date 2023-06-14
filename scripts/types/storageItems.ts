export interface StorageItems {
  codeIds: {
    'red-bank'?: number
    'rewards-collector'?: number
    'address-provider'?: number
    incentives?: number
    oracle?: number
    swapper?: number
    params?: number
  }
  addresses: {
    'address-provider'?: string
    'rewards-collector'?: string
    'red-bank'?: string
    incentives?: string
    oracle?: string
    swapper?: string
    params?: string
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
