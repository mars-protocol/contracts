import {
  LCDClient,
  LocalTerra,
  MnemonicKey,
} from "@terra-money/terra.js"
import { strictEqual, notStrictEqual } from "assert"
import {
  deployContract,
  executeContract, Logger,
  queryContract,
  setGasAdjustment,
  setTimeoutDuration, sleep,
  uploadContract
} from "../helpers.js"
import {
  Asset,
  borrowNative,
  depositNative,
  setAssetOraclePriceSource
} from "./test_helpers.js"

// CONSTS

// Adjust the timeout_* config items in LocalTerra/config/config.toml to 250ms to make the test run faster.
const BLOCK_TIME = 0.25

const USD_COLLATERAL = 100_000_000_000000
const LUNA_COLLATERAL = 100_000_000_000000

// HELPERS

async function queryBorrowRate(
  terra: LCDClient,
  redBank: string,
  asset: Asset,
) {
  const market = await queryContract(terra, redBank, { market: { asset } })
  return parseFloat(market.borrow_rate)
}

// MAIN

(async () => {
  setTimeoutDuration(0)
  // gas is not correctly estimated in the repay_native method on the red bank,
  // so any estimates need to be adjusted upwards
  setGasAdjustment(2)

  const logger = new Logger()

  const terra = new LocalTerra()
  const deployer = terra.wallets.test1
  const user1 = terra.wallets.test2
  const user2 = terra.wallets.test3
  // mock contract addresses
  const protocolRewardsCollector = new MnemonicKey().accAddress

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
        ma_token_code_id: maTokenCodeId,
        close_factor: "0.5",
      }
    }
  )

  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          incentives_address: incentives,
          oracle_address: oracle,
          red_bank_address: redBank,
          protocol_rewards_collector_address: protocolRewardsCollector,
          protocol_admin_address: deployer.key.accAddress,
        }
      }
    },
    {logger: logger}
  )

  console.log("init assets")

  // uluna
  const ulunaCurrentBorrowRate = 0.1
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { native: { denom: "uluna" } },
        asset_params: {
          initial_borrow_rate: ulunaCurrentBorrowRate.toString(),
          max_loan_to_value: "0.60",
          reserve_factor: "0.2",
          liquidation_threshold: "0.80",
          liquidation_bonus: "0.1",
          interest_rate_model_params: {
            dynamic: {
              min_borrow_rate: "0.0",
              max_borrow_rate: "2.0",
              kp_1: "0.02",
              optimal_utilization_rate: "0.7",
              kp_augmentation_threshold: "0.15",
              kp_2: "0.05",
              update_threshold_txs: 2,
              update_threshold_seconds: 600,
            }
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true
        }
      }
    },
    { logger: logger }
  )

  await setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: "uluna" } }, 45, logger)

  // uusd
  let uusdCurrentBorrowRate = 0.2
  const uusdUpdateThresholdSeconds = 40
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { native: { denom: "uusd" } },
        asset_params: {
          initial_borrow_rate: uusdCurrentBorrowRate.toString(),
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
              update_threshold_txs: 3,
              update_threshold_seconds: uusdUpdateThresholdSeconds,
            }
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true
        }
      }
    },
    { logger: logger }
  )

  await setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: "uusd" } }, 1, logger)

  // TESTS

  console.log("[1 tx with usd] deposit")
  await depositNative(terra, user1, redBank, "uusd", USD_COLLATERAL, logger)

  console.log("[1 tx with luna] deposit")
  await depositNative(terra, user2, redBank, "uluna", LUNA_COLLATERAL, logger)

  console.log("[2 tx with usd] borrow")
  await borrowNative(terra, user2, redBank, "uusd", 100_000000, logger)

  // uusd borrow rate should not change
  {
    const uusdBorrowRate = await queryBorrowRate(terra, redBank, { native: { denom: "uusd" } })
    strictEqual(uusdBorrowRate, uusdCurrentBorrowRate)
  }

  const localTerraSec = uusdUpdateThresholdSeconds * BLOCK_TIME
  const timoutInMs = localTerraSec * 1000
  console.log("Wait ~", localTerraSec, " seconds")
  await sleep(timoutInMs + 10)

  console.log("[3 tx with usd] borrow")
  await borrowNative(terra, user2, redBank, "uusd", 200_000000, logger)

  // uusd borrow rate should change because we exceeded threshold txs
  {
    const uusdBorrowRate = await queryBorrowRate(terra, redBank, { native: { denom: "uusd" } })
    notStrictEqual(uusdBorrowRate, uusdCurrentBorrowRate)
    uusdCurrentBorrowRate = uusdBorrowRate
  }

  console.log("[4 tx with usd] borrow")
  await borrowNative(terra, user2, redBank, "uusd", 150_000000, logger)

  // uusd and uluna borrow rates should not change
  {
    const uusdBorrowRate = await queryBorrowRate(terra, redBank, { native: { denom: "uusd" } })
    strictEqual(uusdBorrowRate, uusdCurrentBorrowRate)

    const ulunaBorrowRate = await queryBorrowRate(terra, redBank, { native: { denom: "uluna" } })
    strictEqual(ulunaBorrowRate, ulunaCurrentBorrowRate)
  }

  console.log("[2 tx with luna] borrow")
  await borrowNative(terra, user1, redBank, "uluna", 300_000000, logger)

  // uluna borrow rate should change because we exceeded threshold txs
  {
    const ulunaBorrowRate = await queryBorrowRate(terra, redBank, { native: { denom: "uluna" } })
    notStrictEqual(ulunaBorrowRate, ulunaCurrentBorrowRate)
  }

  console.log("OK")

  logger.showGasConsumption()
})()
