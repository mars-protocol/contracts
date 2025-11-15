export interface LocalnetAsset {
  denom: string
  description?: string
}

export interface PoolToken {
  denom: string
  amount: string
}

export interface PoolConfig {
  name: string
  token1: PoolToken
  token2: PoolToken
  swap_fee: string
  exit_fee: string
}

export interface Balance {
  denom: string
  amount: string
}

export interface SeedAddress {
  address: string
  name?: string
  balances: Balance[]
}

export interface GenesisAccount {
  name: string
  mnemonic: string
  balances: Balance[]
}

export interface ChainConfig {
  chain_id: string
  denom: string
}

export interface LocalnetConfig {
  chain: ChainConfig
  assets: LocalnetAsset[]
  pools: PoolConfig[]
  seed_addresses: SeedAddress[]
  genesis_account: GenesisAccount
}
