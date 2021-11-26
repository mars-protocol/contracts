export const testnet: Config = {
  councilInitMsg: {
    "config": {
      "address_provider_address": undefined,
      "proposal_voting_period": 80, // 20 blocks = ~2.5 minutes (for internal testing) // 57600 blocks = ~5 days
      "proposal_effective_delay": 0, // 0 blocks = able to execute proposal immediately (for internal testing) // 11520 blocks = ~24 hours
      "proposal_expiration_period": 115200, // 115200 blocks = ~10 days
      "proposal_required_deposit": "100000000",
      "proposal_required_quorum": "0.1",
      "proposal_required_threshold": "0.5"
    }
  },
  vestingInitMsg: {
    "address_provider_address": undefined,
    "default_unlock_schedule": {
      "start_time": 1638316800,
      "cliff": 15770000,
      "duration": 94610000
    }
  },
  stakingInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "astroport_factory_address": "terra1q5fku2rf8mcdjz4ud9rsjf2srcd9mhz2d7mwxw",
      "astroport_max_spread": "0.05",
      "cooldown_duration": 90, // Seconds (for internal testing) // 864000 Seconds = 10 days
      "unstake_window": 300, // Seconds (for internal testing) // 172800 Seconds = 2 days
    }
  },
  safetyFundInitMsg: {
    "owner": undefined,
    "astroport_factory_address": "terra1q5fku2rf8mcdjz4ud9rsjf2srcd9mhz2d7mwxw",
    "astroport_max_spread": "0.05",
  },
  treasuryInitMsg: {
    "owner": undefined,
    "astroport_factory_address": "terra1q5fku2rf8mcdjz4ud9rsjf2srcd9mhz2d7mwxw",
    "astroport_max_spread": "0.05",
  },
  protocolRewardsCollectorInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "safety_fund_fee_share": "0.1",
      "treasury_fee_share": "0.2",
      "astroport_factory_address": "terra1q5fku2rf8mcdjz4ud9rsjf2srcd9mhz2d7mwxw",
      "astroport_max_spread": "0.05",
    }
  },
  redBankInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "ma_token_code_id": undefined,
      "close_factor": "0.5"
    }
  },
  initialAssets: [
    // find contract addresses of CW20's here: https://github.com/terra-project/assets/blob/master/cw20/tokens.json
    {
      symbol: "UST",
      denom: "uusd",
      init_params: {
        initial_borrow_rate: "0.2",
        max_loan_to_value: "0.75",
        reserve_factor: "0.2",
        liquidation_threshold: "0.85",
        liquidation_bonus: "0.1",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "1.0",
            kp_1: "0.04",
            optimal_utilization_rate: "0.9",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.07",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
      asset_symbol: "UST",
    },
    {
      symbol: "LUNA",
      denom: "uluna",
      init_params: {
        initial_borrow_rate: "0.1",
        max_loan_to_value: "0.55",
        reserve_factor: "0.2",
        liquidation_threshold: "0.65",
        liquidation_bonus: "0.1",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "2.0",
            kp_1: "0.02",
            optimal_utilization_rate: "0.7",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.05",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
    },
    {
      symbol: "MIR",
      contract_addr: "terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u",
      init_params: {
        initial_borrow_rate: "0.07",
        max_loan_to_value: "0.45",
        reserve_factor: "0.2",
        liquidation_threshold: "0.55",
        liquidation_bonus: "0.15",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "2.0",
            kp_1: "0.02",
            optimal_utilization_rate: "0.5",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.05",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
    },
    {
      symbol: "ANC",
      contract_addr: "terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc",
      init_params: {
        initial_borrow_rate: "0.07",
        max_loan_to_value: "0.35",
        reserve_factor: "0.2",
        liquidation_threshold: "0.45",
        liquidation_bonus: "0.15",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "2.0",
            kp_1: "0.02",
            optimal_utilization_rate: "0.5",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.05",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
    },
    {
      symbol: "MARS",
      contract_addr: "terra1qs7h830ud0a4hj72yr8f7jmlppyx7z524f7gw6",
      init_params: {
        initial_borrow_rate: "0.07",
        max_loan_to_value: "0.45",
        reserve_factor: "0.2",
        liquidation_threshold: "0.55",
        liquidation_bonus: "0.15",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "2.0",
            kp_1: "0.02",
            optimal_utilization_rate: "0.5",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.05",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
    },
    {
      symbol: "MINE",
      contract_addr: "terra1lqm5tutr5xcw9d5vc4457exa3ghd4sr9mzwdex",
      init_params: {
        initial_borrow_rate: "0.07",
        max_loan_to_value: "0.35",
        reserve_factor: "0.2",
        liquidation_threshold: "0.45",
        liquidation_bonus: "0.15",
        interest_rate_model_params: {
          dynamic: {
            min_borrow_rate: "0.0",
            max_borrow_rate: "2.0",
            kp_1: "0.02",
            optimal_utilization_rate: "0.5",
            kp_augmentation_threshold: "0.15",
            kp_2: "0.05",
            update_threshold_txs: 10,
            update_threshold_seconds: 3600,
          }
        },
        active: true,
        deposit_enabled: true,
        borrow_enabled: true
      }
    },
  ],
  mirFarmingStratContractAddress: undefined,
  ancFarmingStratContractAddress: undefined,
  marsFarmingStratContractAddress: undefined,
  minterProxyContractAddress: "terra1hfyg0tvuqd5kk4un4luqng2adc88lgt5skxmve",
  marsTokenContractAddress: "terra1qs7h830ud0a4hj72yr8f7jmlppyx7z524f7gw6",
  oracleFactoryAddress: "terra1q5fku2rf8mcdjz4ud9rsjf2srcd9mhz2d7mwxw",
}

export const local: Config = {
  councilInitMsg: {
    "config": {
      "address_provider_address": undefined,

      "proposal_voting_period": 1000,
      "proposal_effective_delay": 150,
      "proposal_expiration_period": 3000,
      "proposal_required_deposit": "100000000",
      "proposal_required_quorum": "0.1",
      "proposal_required_threshold": "0.5"
    }
  },
  vestingInitMsg: {
    // "config": {
    "address_provider_address": undefined,
    "default_unlock_schedule": {
      "start_time": 1638316800,
      "cliff": 15770000,
      "duration": 94610000
    }
    // }
  },
  stakingInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "astroport_factory_address": undefined,
      "astroport_max_spread": "0.05",
      "cooldown_duration": 10,
      "unstake_window": 300,
    }
  },
  safetyFundInitMsg: {
    "owner": undefined,
    "astroport_factory_address": undefined,
    "astroport_max_spread": "0.05",
  },
  treasuryInitMsg: {
    "owner": undefined,
    "astroport_factory_address": undefined,
    "astroport_max_spread": "0.05",
  },
  protocolRewardsCollectorInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "safety_fund_fee_share": "0.1",
      "treasury_fee_share": "0.2",
      "astroport_factory_address": undefined,
      "astroport_max_spread": "0.05",
    }
  },
  redBankInitMsg: {
    "config": {
      "owner": undefined,
      "address_provider_address": undefined,
      "ma_token_code_id": undefined,
      "close_factor": "0.5"
    }
  },
  initialAssets: [],
  mirFarmingStratContractAddress: undefined,
  ancFarmingStratContractAddress: undefined,
  marsFarmingStratContractAddress: undefined,
  minterProxyContractAddress: undefined,
  marsTokenContractAddress: undefined,
  oracleFactoryAddress: undefined,
}
