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
    'address-provider-updated'?: boolean
    'assets-initialized': string[]
    'second-asset-initialized'?: boolean
    'oracle-price-set'?: boolean
    'smoke-test'?: boolean
  }

  owner?: string
}
