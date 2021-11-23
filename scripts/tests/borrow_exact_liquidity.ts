import { LocalTerra } from "@terra-money/terra.js"
import { join } from "path"
import 'dotenv/config.js'
import {
  deployContract,
  executeContract,
  setTimeoutDuration,
  uploadContract
} from "../helpers.js"
import {
  borrowNative,
  depositCw20,
  depositNative,
  setAssetOraclePriceSource
} from "./test_helpers.js"

// CONSTS

// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!

const UUSD_COLLATERAL = 1_000_000_000000
const MARS_COLLATERAL = 100_000_000_000000;

// MAIN

(async () => {
  setTimeoutDuration(0)

  const terra = new LocalTerra()
  const deployer = terra.wallets.test1
  const provider = terra.wallets.test2
  const borrower = terra.wallets.test3

  console.log("upload contracts")

  const addressProvider = await deployContract(terra, deployer, "../artifacts/mars_address_provider.wasm",
    { owner: deployer.key.accAddress }
  )

  const incentives = await deployContract(terra, deployer, "../artifacts/mars_incentives.wasm",
    {
      owner: deployer.key.accAddress,
      address_provider_address: addressProvider
    }
  )

  const oracle = await deployContract(terra, deployer, "../artifacts/mars_oracle.wasm",
    { owner: deployer.key.accAddress }
  )

  const maTokenCodeId = await uploadContract(terra, deployer, "../artifacts/mars_ma_token.wasm")

  const redBank = await deployContract(terra, deployer, "../artifacts/mars_red_bank.wasm",
    {
      config: {
        owner: deployer.key.accAddress,
        address_provider_address: addressProvider,
        safety_fund_fee_share: "0.1",
        treasury_fee_share: "0.2",
        ma_token_code_id: maTokenCodeId,
        close_factor: "0.5",
      }
    }
  )

  const mars = await deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm"),
    {
      name: "Mars",
      symbol: "MARS",
      decimals: 6,
      initial_balances: [{ address: borrower.key.accAddress, amount: String(MARS_COLLATERAL) }],
    }
  )

  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          incentives_address: incentives,
          mars_token_address: mars,
          oracle_address: oracle,
          red_bank_address: redBank,
          protocol_admin_address: deployer.key.accAddress,
        }
      }
    }
  )

  console.log("init assets")

  // mars
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { cw20: { contract_addr: mars } },
        asset_params: {
          initial_borrow_rate: "0.1",
          max_loan_to_value: "0.55",
          reserve_factor: "0.2",
          liquidation_threshold: "0.65",
          liquidation_bonus: "0.1",
          interest_rate_model_params: {
            linear: {
              optimal_utilization_rate: "1",
              base: "0",
              slope_1: "1",
              slope_2: "0",
            }
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true
        }
      }
    }
  )

  await setAssetOraclePriceSource(terra, deployer, oracle,
    { cw20: { contract_addr: mars } },
    2
  )

  // uusd
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { native: { denom: "uusd" } },
        asset_params: {
          initial_borrow_rate: "0.2",
          max_loan_to_value: "0.75",
          reserve_factor: "0.2",
          liquidation_threshold: "0.85",
          liquidation_bonus: "0.1",
          interest_rate_model_params: {
            linear: {
              optimal_utilization_rate: "1",
              base: "0",
              slope_1: "1",
              slope_2: "0",
            }
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true
        }
      }
    }
  )

  await setAssetOraclePriceSource(terra, deployer, oracle,
    { native: { denom: "uusd" } },
    1
  )

  // TESTS

  console.log("provide uusd")

  await depositNative(terra, provider, redBank, "uusd", UUSD_COLLATERAL)

  console.log("provide mars")

  await depositCw20(terra, borrower, redBank, mars, MARS_COLLATERAL)

  console.log("borrow uusd")

  await borrowNative(terra, borrower, redBank, "uusd", UUSD_COLLATERAL)

  console.log("OK")
})()
