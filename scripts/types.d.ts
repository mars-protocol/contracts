interface CouncilInitMsg {
  config: {
    address_provider_address?: string
    proposal_voting_period: number
    proposal_effective_delay: number
    proposal_expiration_period: number
    proposal_required_deposit: string
    proposal_required_quorum: string
    proposal_required_threshold: string
  }
}

interface VestingInitMsg {
  address_provider_address?: string
  default_unlock_schedule: Schedule
}

interface Schedule {
  start_time: number
  cliff: number
  duration: number
}

interface StakingInitMsg {
  config: {
    owner?: string
    address_provider_address?: string
    astroport_factory_address?: string
    astroport_max_spread: string
    cooldown_duration: number
    unstake_window: number
  }
}

interface SafetyFundInitMsg {
  owner?: string
  astroport_factory_address?: string
  astroport_max_spread: string
}

interface TreasuryInitMsg {
  owner?: string
  astroport_factory_address?: string
  astroport_max_spread: string
}

interface ProtocolRewardsCollectorInitMsg {
  config: {
    owner?: string,
    address_provider_address?: string,
    safety_fund_fee_share: string,
    treasury_fee_share: string,
    astroport_factory_address?: string,
    astroport_max_spread: string,
  }
}

interface RedBankInitMsg {
  config: {
    owner?: string
    address_provider_address?: string
    ma_token_code_id?: number
    close_factor: string
  }
}

interface DynamicInterestRate {
  dynamic: {
    min_borrow_rate: string
    max_borrow_rate: string
    kp_1: string
    optimal_utilization_rate: string
    kp_augmentation_threshold: string
    kp_2: string
    update_threshold_txs: number
    update_threshold_seconds: number
  }
}

interface LinearInterestRate {
  linear: {
    optimal_utilization_rate: string
    base: string
    slope_1: string
    slope_2: string
  }
}

interface InitOrUpdateAssetParams {
  initial_borrow_rate: string
  max_loan_to_value: string
  reserve_factor: string
  liquidation_threshold: string
  liquidation_bonus: string
  interest_rate_model_params: DynamicInterestRate | LinearInterestRate
  active: boolean
  deposit_enabled: boolean
  borrow_enabled: boolean
}

interface Asset {
  denom?: string
  symbol?: string
  contract_addr?: string
  init_params: InitOrUpdateAssetParams
}

interface Config {
  councilInitMsg: CouncilInitMsg
  vestingInitMsg: VestingInitMsg
  stakingInitMsg: StakingInitMsg
  safetyFundInitMsg: SafetyFundInitMsg
  treasuryInitMsg: TreasuryInitMsg
  protocolRewardsCollectorInitMsg: ProtocolRewardsCollectorInitMsg
  redBankInitMsg: RedBankInitMsg
  initialAssets: Asset[]
  mirFarmingStratContractAddress: string | undefined
  ancFarmingStratContractAddress: string | undefined
  marsFarmingStratContractAddress: string | undefined
  minterProxyContractAddress: string | undefined
  marsTokenContractAddress: string | undefined
  oracleFactoryAddress: string | undefined
}
