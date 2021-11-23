import { LocalTerra } from "@terra-money/terra.js"
import { strictEqual, strict as assert } from "assert"
import {
  deployContract,
  executeContract,
  queryContract,
  setGasAdjustment,
  setTimeoutDuration,
  sleep,
  uploadContract
} from "../helpers.js"
import {
  borrowNative,
  depositNative,
  getTxTimestamp,
  queryBalanceCw20,
  queryMaAssetAddress,
  setAssetOraclePriceSource,
} from "./test_helpers.js"

// CONSTS

const USD_COLLATERAL = 1000_000000
const LUNA_COLLATERAL = 1000_000000
const USD_BORROW = 1000_000000

const RESERVE_FACTOR = 0.2 // 20%
const INTEREST_RATE = 0.25 // 25% pa

const SECONDS_IN_YEAR = 60 * 60 * 24 * 365;

// MAIN

(async () => {
  setTimeoutDuration(100)
  // gas is not correctly estimated in the repay_native method on the red bank,
  // so any estimates need to be adjusted upwards
  setGasAdjustment(2)

  const terra = new LocalTerra()
  const deployer = terra.wallets.test1
  const provider = terra.wallets.test2
  const borrower = terra.wallets.test3
  // mock contract addresses
  const protocolRewardsCollector = terra.wallets.test10.key.accAddress

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

  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          incentives_address: incentives,
          oracle_address: oracle,
          red_bank_address: redBank,
          protocol_rewards_collector: protocolRewardsCollector,
          protocol_admin_address: deployer.key.accAddress,
        }
      }
    }
  )

  console.log("init assets")

  // uluna
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { native: { denom: "uluna" } },
        asset_params: {
          initial_borrow_rate: "0.1",
          max_loan_to_value: "0.55",
          reserve_factor: "0.2",
          liquidation_threshold: "0.65",
          liquidation_bonus: "0.1",
          interest_rate_model_params: {
            linear: {
              optimal_utilization_rate: "0",
              base: "1",
              slope_1: "0",
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

  await setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: "uluna" } }, 25)

  // uusd
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { native: { denom: "uusd" } },
        asset_params: {
          initial_borrow_rate: "0.2",
          max_loan_to_value: "0.75",
          reserve_factor: String(RESERVE_FACTOR),
          liquidation_threshold: "0.85",
          liquidation_bonus: "0.1",
          interest_rate_model_params: {
            linear: {
              optimal_utilization_rate: "0",
              base: String(INTEREST_RATE),
              slope_1: "0",
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
  const maUusd = await queryMaAssetAddress(terra, redBank, { native: { denom: "uusd" } })

  await setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: "uusd" } }, 1)

  // TESTS

  console.log("provide usd")

  await depositNative(terra, provider, redBank, "uusd", USD_COLLATERAL)

  // underlying_liquidity_amount will be the same as the amount of uusd provided
  const maUusdBalance = await queryBalanceCw20(terra, provider.key.accAddress, maUusd)
  const underlyingLiquidityAmount = parseInt(
    await queryContract(terra, redBank,
      {
        underlying_liquidity_amount: {
          ma_token_address: maUusd,
          amount_scaled: String(maUusdBalance)
        }
      }
    )
  )
  strictEqual(underlyingLiquidityAmount, USD_COLLATERAL)

  console.log("provide luna")

  await depositNative(terra, borrower, redBank, "uluna", LUNA_COLLATERAL)

  console.log("borrow")

  let result = await borrowNative(terra, borrower, redBank, "uusd", USD_BORROW)
  const borrowTime = await getTxTimestamp(terra, result)

  await sleep(1000)

  console.log("check interest accrues")

  let prevUnderlyingLiquidityAmount = USD_COLLATERAL
  for (const i of Array(5).keys()) {
    const maUusdBalance = await queryBalanceCw20(terra, provider.key.accAddress, maUusd)
    const underlyingLiquidityAmount = parseInt(
      await queryContract(terra, redBank,
        {
          underlying_liquidity_amount: {
            ma_token_address: maUusd,
            amount_scaled: String(maUusdBalance)
          }
        }
      )
    )

    // manually calculate accrued interest
    const block = await terra.tendermint.blockInfo()
    const blockTime = Math.floor(Date.parse(block.block.header.time) / 1000)
    const elapsed = blockTime - borrowTime

    const utilizationRate = USD_BORROW / USD_COLLATERAL
    const fractionOfYear = elapsed / SECONDS_IN_YEAR
    const liquidityInterestRate = INTEREST_RATE * fractionOfYear * utilizationRate * (1 - RESERVE_FACTOR)
    const amountWithInterest = Math.floor(USD_COLLATERAL * (1 + liquidityInterestRate))

    strictEqual(amountWithInterest, underlyingLiquidityAmount)

    // check underlying liquidity amount increases with time due to interest accruing
    assert(underlyingLiquidityAmount > prevUnderlyingLiquidityAmount)
    prevUnderlyingLiquidityAmount = underlyingLiquidityAmount

    await sleep(1000)
  }

  console.log("OK")
})()
