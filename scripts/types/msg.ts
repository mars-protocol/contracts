export type InstantiateMsgs =
  | RedBankInstantiateMsg
  | AddressProviderInstantiateMsg
  | IncentivesInstantiateMsg
  | OracleInstantiateMsg
  | RewardsInstantiateMsg

export interface RedBankInstantiateMsg {
  owner: string
  emergency_owner: string
  config: {
    address_provider: string
    close_factor: string
  }
}

export interface AddressProviderInstantiateMsg {
  owner: string
  prefix: string
}

export interface IncentivesInstantiateMsg {
  owner: string
  address_provider: string
  mars_denom: string
}

export interface OracleInstantiateMsg {
  owner: string
  base_denom: string
}

export interface RewardsInstantiateMsg {
  owner: string
  safety_fund_denom: string
  address_provider: string
  slippage_tolerance: string
  safety_tax_rate: string
  timeout_seconds: number
  fee_collector_denom: string
  channel_id: string
}

export interface UpdateOwner {
  owner: string
}
