import {
  Coin,
  LocalTerra,
  MnemonicKey,
  Wallet
} from "@terra-money/terra.js"
import {
  strictEqual,
  strict as assert
} from "assert"
import { join } from "path"
import 'dotenv/config.js'
import {
  deployContract,
  executeContract,
  instantiateContract, Logger,
  queryContract,
  setGasAdjustment,
  setTimeoutDuration,
  sleep,
  toEncodedBinary,
  uploadContract
} from "../helpers.js"
import {
  borrowCw20,
  borrowNative,
  deductTax,
  depositCw20,
  depositNative,
  mintCw20,
  queryBalanceCw20,
  queryMaAssetAddress,
  queryBalanceNative,
  setAssetOraclePriceSource,
  approximateEqual
} from "./test_helpers.js"

// CONSTS

// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH!

// protocol rewards collector
const SAFETY_FUND_FEE_SHARE = 0.1
const TREASURY_FEE_SHARE = 0.2

// red-bank
const CLOSE_FACTOR = 0.5
const MAX_LTV = 0.55
const LIQUIDATION_BONUS = 0.1
const MA_TOKEN_SCALING_FACTOR = 1_000_000
// set a high interest rate, so tests can be run faster
const INTEREST_RATE = 100000

// native tokens
const LUNA_USD_PRICE = 25
const USD_COLLATERAL_AMOUNT = 100_000_000_000000
const LUNA_COLLATERAL_AMOUNT = 1_000_000000
const USD_BORROW_AMOUNT = LUNA_COLLATERAL_AMOUNT * LUNA_USD_PRICE * MAX_LTV

// cw20 tokens
const CW20_TOKEN_USD_PRICE = 10
const CW20_TOKEN_1_COLLATERAL_AMOUNT = 100_000_000_000000
const CW20_TOKEN_2_COLLATERAL_AMOUNT = 1_000_000000
const CW20_TOKEN_1_BORROW_AMOUNT = CW20_TOKEN_2_COLLATERAL_AMOUNT * MAX_LTV
const CW20_TOKEN_1_UUSD_PAIR_UUSD_LP_AMOUNT = 1_000_000_000000
const CW20_TOKEN_1_UUSD_PAIR_CW20_TOKEN_1_LP_AMOUNT = CW20_TOKEN_1_UUSD_PAIR_UUSD_LP_AMOUNT * CW20_TOKEN_USD_PRICE

// TYPES

interface Env {
  terra: LocalTerra
  deployer: Wallet
  provider: Wallet
  borrower: Wallet
  cw20Token1: string
  cw20Token2: string
  maUluna: string
  maUusd: string
  maCw20Token1: string
  maCw20Token2: string
  redBank: string
  protocolRewardsCollector: string
  treasury: string
  safetyFund: string
  staking: string
}

// TESTS

async function testNative(env: Env, logger?: Logger) {
  const {
    terra,
    deployer,
    provider,
    borrower,
    maUusd,
    redBank,
    protocolRewardsCollector,
    treasury,
    safetyFund,
    staking
  } = env

  {
    console.log("provider provides uusd")

    const maUusdBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    strictEqual(maUusdBalanceBefore, 0)

    await depositNative(terra, provider, redBank, "uusd", USD_COLLATERAL_AMOUNT)

    const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    strictEqual(maUusdBalanceAfter, 0)
  }

  console.log("borrower provides uluna")

  await depositNative(terra, borrower, redBank, "uluna", LUNA_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows uusd up to the borrow limit of their uluna collateral")

  await borrowNative(terra, borrower, redBank, "uusd", Math.floor(USD_BORROW_AMOUNT), logger)

  {
    console.log("repay")

    const maUusdBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)

    await executeContract(terra, borrower, redBank,
      { repay_native: { denom: "uusd" } },
      { coins: `${Math.floor(USD_BORROW_AMOUNT)}uusd`, logger: logger }
    )

    const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    assert(maUusdBalanceAfter > maUusdBalanceBefore)
  }

  {
    console.log("withdraw")

    const maUusdBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)

    await executeContract(terra, provider, redBank,
      {
        withdraw: {
          asset: { native: { denom: "uusd" } },
          amount: String(Math.floor(USD_COLLATERAL_AMOUNT / 2))
        }
      },
      { logger: logger }
    )

    const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    assert(maUusdBalanceAfter > maUusdBalanceBefore)
  }

  console.log("protocol rewards collector withdraws from the red bank")

  {
    console.log("- specify an amount")

    const maUusdBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    const uusdBalanceBefore = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")

    // withdraw half of the deposited balance
    await executeContract(terra, deployer, protocolRewardsCollector,
      {
        withdraw_from_red_bank: {
          asset: { native: { denom: "uusd" } },
          amount: String(Math.floor(maUusdBalanceBefore / MA_TOKEN_SCALING_FACTOR / 2))
        }
      },
      { logger: logger }
    )

    const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    const uusdBalanceAfter = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    assert(maUusdBalanceAfter < maUusdBalanceBefore)
    assert(uusdBalanceAfter > uusdBalanceBefore)
  }

  {
    console.log("- don't specify an amount")

    const uusdBalanceBefore = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")

    // withdraw remaining balance
    let result = await executeContract(terra, deployer, protocolRewardsCollector,
      { withdraw_from_red_bank: { asset: { native: { denom: "uusd" } } } }, { logger: logger }
    )

    const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
    const uusdBalanceAfter = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    assert(uusdBalanceAfter > uusdBalanceBefore)

    // withdrawing from the red bank triggers protocol rewards to be minted to the protocol rewards
    // collector, so the maUusd balance will not be zero after this call
    const maUusdMintAmount = parseInt(result.logs[0].eventsByType.wasm.amount[0])
    strictEqual(maUusdBalanceAfter, maUusdMintAmount)
  }

  console.log("try to distribute uusd rewards")

  await assert.rejects(
    executeContract(terra, deployer, protocolRewardsCollector,
      { distribute_protocol_rewards: { asset: { native: { denom: "uusd" } } } }, { logger: logger }
    ),
    (error: any) => {
      return error.response.data.message.includes("Asset is not enabled for distribution: \"uusd\"")
    }
  )

  console.log("enable uusd for distribution")

  await executeContract(terra, deployer, protocolRewardsCollector,
    {
      update_asset_config: {
        asset: { native: { denom: "uusd" } },
        enabled: true
      }
    },
    { logger: logger }
  )

  {
    console.log("distribute uusd rewards")

    const protocolRewardsCollectorUusdBalanceBefore = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    const treasuryUusdBalanceBefore = await queryBalanceNative(terra, treasury, "uusd")
    const safetyFundUusdBalanceBefore = await queryBalanceNative(terra, safetyFund, "uusd")
    const stakingUusdBalanceBefore = await queryBalanceNative(terra, staking, "uusd")

    await executeContract(terra, deployer, protocolRewardsCollector,
      { distribute_protocol_rewards: { asset: { native: { denom: "uusd" } } } }, { logger: logger }
    )

    const protocolRewardsCollectorUusdBalanceAfter = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    const treasuryUusdBalanceAfter = await queryBalanceNative(terra, treasury, "uusd")
    const safetyFundUusdBalanceAfter = await queryBalanceNative(terra, safetyFund, "uusd")
    const stakingUusdBalanceAfter = await queryBalanceNative(terra, staking, "uusd")

    // Check a tight interval instead of equality for safety fund, treasury and staking transfer errors
    approximateEqual(protocolRewardsCollectorUusdBalanceAfter, 0, 3)

    const protocolRewardsCollectorUusdBalanceDifference =
      protocolRewardsCollectorUusdBalanceBefore - protocolRewardsCollectorUusdBalanceAfter
    const treasuryUusdBalanceDifference = treasuryUusdBalanceAfter - treasuryUusdBalanceBefore
    const safetyFundUusdBalanceDifference = safetyFundUusdBalanceAfter - safetyFundUusdBalanceBefore
    const stakingUusdBalanceDifference = stakingUusdBalanceAfter - stakingUusdBalanceBefore

    const expectedTreasuryUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * TREASURY_FEE_SHARE)
      )).toNumber()
    const expectedSafetyFundUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * SAFETY_FUND_FEE_SHARE)
      )).toNumber()

    const expectedStakingUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * (1 - (TREASURY_FEE_SHARE + SAFETY_FUND_FEE_SHARE)))
      )).toNumber()

    // Check a tight interval instead of equality for calculating the split + transfer error
    approximateEqual(treasuryUusdBalanceDifference, expectedTreasuryUusdBalanceDifference, 2)
    // Check a tight interval instead of equality for calculating the split + transfer error
    approximateEqual(safetyFundUusdBalanceDifference, expectedSafetyFundUusdBalanceDifference, 2)
    // Check a tight interval instead of equality for calculating the safety fund and treasury splits
    // + transfer error
    approximateEqual(stakingUusdBalanceDifference, expectedStakingUusdBalanceDifference, 4)
  }
}

async function testCw20(env: Env, logger?: Logger) {
  const {
    terra,
    deployer,
    provider,
    borrower,
    cw20Token1,
    cw20Token2,
    maCw20Token1,
    redBank,
    protocolRewardsCollector,
    treasury,
    safetyFund,
    staking,
  } = env

  // mint some tokens
  await mintCw20(terra, deployer, cw20Token1, provider.key.accAddress, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)
  await mintCw20(terra, deployer, cw20Token2, borrower.key.accAddress, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)

  {
    console.log("provider provides cw20 token 1")

    const maCwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    strictEqual(maCwToken1BalanceBefore, 0)

    await depositCw20(terra, provider, redBank, cw20Token1, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)

    const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    strictEqual(maCwToken1BalanceAfter, 0)
  }

  console.log("borrower provides cw20 token 2")

  await depositCw20(terra, borrower, redBank, cw20Token2, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows cw20 token 1 up to the borrow limit of their cw20 token 2 collateral")

  await borrowCw20(terra, borrower, redBank, cw20Token1, CW20_TOKEN_1_BORROW_AMOUNT, logger)

  {
    console.log("repay")

    const maCwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)

    await executeContract(terra, borrower, cw20Token1,
      {
        send: {
          contract: redBank,
          amount: String(CW20_TOKEN_1_BORROW_AMOUNT),
          msg: toEncodedBinary({ repay_cw20: {} })
        }
      },
      { logger: logger }
    )

    const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    assert(maCwToken1BalanceAfter > maCwToken1BalanceBefore)
  }

  {
    console.log("withdraw")

    const maCwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)

    await executeContract(terra, provider, redBank,
      {
        withdraw: {
          asset: { cw20: { contract_addr: cw20Token1 } },
          amount: String(Math.floor(CW20_TOKEN_1_BORROW_AMOUNT / 2))
        }
      },
      { logger: logger }
    )

    const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    assert(maCwToken1BalanceAfter > maCwToken1BalanceBefore)
  }

  console.log("protocol rewards collector withdraws from the red bank")

  {
    console.log("- specify an amount")

    const maCwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    const cwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, cw20Token1)

    // withdraw half of the deposited balance
    await executeContract(terra, deployer, protocolRewardsCollector,
      {
        withdraw_from_red_bank: {
          asset: { cw20: { contract_addr: cw20Token1 } },
          amount: String(Math.floor(maCwToken1BalanceBefore / MA_TOKEN_SCALING_FACTOR / 2))
        }
      },
      { logger: logger }
    )

    const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    const cwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, cw20Token1)
    assert(maCwToken1BalanceAfter < maCwToken1BalanceBefore)
    assert(cwToken1BalanceAfter > cwToken1BalanceBefore)
  }

  {
    console.log("- don't specify an amount")

    const cwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, cw20Token1)

    // withdraw remaining balance
    const result = await executeContract(terra, deployer, protocolRewardsCollector,
      { withdraw_from_red_bank: { asset: { cw20: { contract_addr: cw20Token1 } } } }, { logger: logger }
    )

    const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
    const cwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, cw20Token1)
    assert(cwToken1BalanceAfter > cwToken1BalanceBefore)

    // withdrawing from the red bank triggers protocol rewards to be minted to the protocol rewards
    // collector, so the maCw20Token1 balance will not be zero after this call
    const maCw20Token1MintAmount = parseInt(result.logs[0].eventsByType.wasm.amount[0])
    strictEqual(maCwToken1BalanceAfter, maCw20Token1MintAmount)
  }

  console.log("try to distribute cw20 token 1 rewards")

  await assert.rejects(
    executeContract(terra, deployer, protocolRewardsCollector,
      { distribute_protocol_rewards: { asset: { cw20: { contract_addr: cw20Token1 } } } }, { logger: logger }
    ),
    (error: any) => {
      return error.response.data.message.includes(`Asset is not enabled for distribution: \"${cw20Token1}\"`)
    }
  )

  console.log("swap cw20 token 1 to uusd")

  await executeContract(terra, deployer, protocolRewardsCollector,
    { swap_asset_to_uusd: { offer_asset_info: { token: { contract_addr: cw20Token1 } } } }, { logger: logger }
  )

  console.log("enable uusd for distribution")

  await executeContract(terra, deployer, protocolRewardsCollector,
    {
      update_asset_config: {
        asset: { native: { denom: "uusd" } },
        enabled: true
      }
    },
    { logger: logger }
  )

  {
    console.log("distribute uusd rewards")

    const protocolRewardsCollectorUusdBalanceBefore = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    const treasuryUusdBalanceBefore = await queryBalanceNative(terra, treasury, "uusd")
    const safetyFundUusdBalanceBefore = await queryBalanceNative(terra, safetyFund, "uusd")
    const stakingUusdBalanceBefore = await queryBalanceNative(terra, staking, "uusd")

    await executeContract(terra, deployer, protocolRewardsCollector,
      { distribute_protocol_rewards: { asset: { native: { denom: "uusd" } } } }, { logger: logger }
    )

    const protocolRewardsCollectorUusdBalanceAfter = await queryBalanceNative(terra, protocolRewardsCollector, "uusd")
    const treasuryUusdBalanceAfter = await queryBalanceNative(terra, treasury, "uusd")
    const safetyFundUusdBalanceAfter = await queryBalanceNative(terra, safetyFund, "uusd")
    const stakingUusdBalanceAfter = await queryBalanceNative(terra, staking, "uusd")

    // Check a tight interval instead of equality for safety fund, treasury and staking transfer errors
    approximateEqual(protocolRewardsCollectorUusdBalanceAfter, 0, 3)

    const protocolRewardsCollectorUusdBalanceDifference =
      protocolRewardsCollectorUusdBalanceBefore - protocolRewardsCollectorUusdBalanceAfter
    const treasuryUusdBalanceDifference = treasuryUusdBalanceAfter - treasuryUusdBalanceBefore
    const safetyFundUusdBalanceDifference = safetyFundUusdBalanceAfter - safetyFundUusdBalanceBefore
    const stakingUusdBalanceDifference = stakingUusdBalanceAfter - stakingUusdBalanceBefore

    const expectedTreasuryUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * TREASURY_FEE_SHARE)
      )).toNumber()
    const expectedSafetyFundUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * SAFETY_FUND_FEE_SHARE)
      )).toNumber()

    const expectedStakingUusdBalanceDifference =
      (await deductTax(
        terra,
        new Coin("uusd", protocolRewardsCollectorUusdBalanceDifference * (1 - (TREASURY_FEE_SHARE + SAFETY_FUND_FEE_SHARE)))
      )).toNumber()

    // Check a tight interval instead of equality for calculating the split + transfer error
    approximateEqual(treasuryUusdBalanceDifference, expectedTreasuryUusdBalanceDifference, 2)
    // Check a tight interval instead of equality for calculating the split + transfer error
    approximateEqual(safetyFundUusdBalanceDifference, expectedSafetyFundUusdBalanceDifference, 2)
    // Check a tight interval instead of equality for calculating the safety fund and treasury splits
    // + transfer error
    approximateEqual(stakingUusdBalanceDifference, expectedStakingUusdBalanceDifference, 4)
  }
}

async function testLiquidateNative(env: Env, logger?: Logger) {
  const {
    terra,
    deployer,
    provider,
    borrower,
    maUluna,
    maUusd,
    redBank,
    protocolRewardsCollector,
  } = env

  const liquidator = deployer

  console.log("provider provides uusd")

  await depositNative(terra, provider, redBank, "uusd", USD_COLLATERAL_AMOUNT, logger)

  console.log("borrower provides uluna")

  await depositNative(terra, borrower, redBank, "uluna", LUNA_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows uusd up to the borrow limit of their uluna collateral")

  await borrowNative(terra, borrower, redBank, "uusd", Math.floor(USD_BORROW_AMOUNT), logger)

  console.log("someone borrows uluna in order for rewards to start accruing")

  await borrowNative(terra, provider, redBank, "uluna", Math.floor(LUNA_COLLATERAL_AMOUNT / 10), logger)

  console.log("liquidator waits until the borrower's health factor is < 1, then liquidates")

  // wait until the borrower can be liquidated
  let tries = 0
  let maxTries = 10
  let backoff = 1

  while (true) {
    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: borrower.key.accAddress } }
    )
    const healthFactor = parseFloat(userPosition.health_status.borrowing)
    if (healthFactor < 1.0) {
      break
    }

    // timeout
    tries++
    if (tries == maxTries) {
      throw new Error(`timed out waiting ${maxTries} times for the borrower to be liquidated`)
    }

    // exponential backoff
    console.log("health factor:", healthFactor, `backing off: ${backoff} s`)
    await sleep(backoff * 1000)
    backoff *= 2
  }

  // get the protocol rewards collector balances before the borrower is liquidated
  const maUusdBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
  const maUlunaBalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maUluna)

  await executeContract(terra, liquidator, redBank,
    {
      liquidate_native: {
        collateral_asset: { native: { denom: "uluna" } },
        debt_asset_denom: "uusd",
        user_address: borrower.key.accAddress,
        receive_ma_token: false,
      }
    },
    { coins: `${Math.floor(USD_BORROW_AMOUNT * CLOSE_FACTOR)}uusd`, logger: logger }
  )

  // get the protocol rewards collector balances after the borrower is liquidated
  const maUusdBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUusd)
  const maUlunaBalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maUluna)
  assert(maUusdBalanceAfter > maUusdBalanceBefore)
  assert(maUlunaBalanceAfter > maUlunaBalanceBefore)
}

async function testLiquidateCw20(env: Env, logger?: Logger) {
  const {
    terra,
    deployer,
    provider,
    borrower,
    maCw20Token1,
    maCw20Token2,
    cw20Token1,
    cw20Token2,
    redBank,
    protocolRewardsCollector
  } = env

  const liquidator = deployer

  // mint some tokens
  await mintCw20(terra, deployer, cw20Token1, provider.key.accAddress, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)
  await mintCw20(terra, deployer, cw20Token1, liquidator.key.accAddress, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)
  await mintCw20(terra, deployer, cw20Token2, borrower.key.accAddress, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)

  console.log("provider provides cw20 token 1")

  await depositCw20(terra, provider, redBank, cw20Token1, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)

  console.log("borrower provides cw20 token 2")

  await depositCw20(terra, borrower, redBank, cw20Token2, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows cw20 token 1 up to the borrow limit of their cw20 token 2 collateral")

  await borrowCw20(terra, borrower, redBank, cw20Token1, CW20_TOKEN_1_BORROW_AMOUNT, logger)

  console.log("someone borrows cw20 token 2 in order for rewards to start accruing")

  await borrowCw20(terra, provider, redBank, cw20Token2, Math.floor(CW20_TOKEN_1_BORROW_AMOUNT / 10), logger)

  console.log("liquidator waits until the borrower's health factor is < 1, then liquidates")

  // wait until the borrower can be liquidated
  let tries = 0
  let maxTries = 10
  let backoff = 1

  while (true) {
    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: borrower.key.accAddress } }
    )
    const healthFactor = parseFloat(userPosition.health_status.borrowing)
    if (healthFactor < 1.0) {
      break
    }

    // timeout
    tries++
    if (tries == maxTries) {
      throw new Error(`timed out waiting ${maxTries} times for the borrower to be liquidated`)
    }

    // exponential backoff
    console.log("health factor:", healthFactor, `backing off: ${backoff} s`)
    await sleep(backoff * 1000)
    backoff *= 2
  }

  // get the protocol rewards collector balances before the borrower is liquidated
  const maCwToken1BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
  const maCwToken2BalanceBefore = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token2)

  await executeContract(terra, liquidator, cw20Token1,
    {
      send: {
        contract: redBank,
        amount: String(Math.floor(CW20_TOKEN_1_BORROW_AMOUNT * CLOSE_FACTOR)),
        msg: toEncodedBinary({
          liquidate_cw20: {
            collateral_asset: { cw20: { contract_addr: cw20Token2 } },
            user_address: borrower.key.accAddress,
            receive_ma_token: false,
          }
        })
      }
    },
    { logger: logger }
  )

  // get the protocol rewards collector balances after the borrower is liquidated
  const maCwToken1BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token1)
  const maCwToken2BalanceAfter = await queryBalanceCw20(terra, protocolRewardsCollector, maCw20Token2)
  assert(maCwToken1BalanceAfter > maCwToken1BalanceBefore)
  assert(maCwToken2BalanceAfter > maCwToken2BalanceBefore)
}

// MAIN

(async () => {
  setTimeoutDuration(0)
  // gas is not correctly estimated in the repay_native method on the red bank,
  // so any estimates need to be adjusted upwards
  setGasAdjustment(2)

  const logger = new Logger()

  const terra = new LocalTerra()

  // addresses
  const deployer = terra.wallets.test1
  const provider = terra.wallets.test2
  const borrower = terra.wallets.test3
  // mock contract addresses
  const staking = new MnemonicKey().accAddress
  const safetyFund = new MnemonicKey().accAddress
  const treasury = new MnemonicKey().accAddress
  const astroportGenerator = new MnemonicKey().accAddress

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

  const tokenCodeID = await uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_token.wasm"))
  const pairCodeID = await uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_pair.wasm"))
  const astroportFactory = await deployContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_factory.wasm"),
    {
      owner: deployer.key.accAddress,
      token_code_id: tokenCodeID,
      generator_address: astroportGenerator,
      pair_configs: [
        {
          code_id: pairCodeID,
          pair_type: { xyk: {} },
          total_fee_bps: 0,
          maker_fee_bps: 0
        }
      ]
    }
  )

  const protocolRewardsCollector = await deployContract(terra, deployer, "../artifacts/mars_protocol_rewards_collector.wasm",
    {
      config: {
        owner: deployer.key.accAddress,
        address_provider_address: addressProvider,
        safety_fund_fee_share: String(SAFETY_FUND_FEE_SHARE),
        treasury_fee_share: String(TREASURY_FEE_SHARE),
        astroport_factory_address: astroportFactory,
        astroport_max_spread: "0.05",
      }
    }
  )

  // update address provider
  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          protocol_rewards_collector_address: protocolRewardsCollector,
          staking_address: staking,
          treasury_address: treasury,
          safety_fund_address: safetyFund,
          incentives_address: incentives,
          oracle_address: oracle,
          red_bank_address: redBank,
          protocol_admin_address: deployer.key.accAddress,
        }
      }
    },
    { logger: logger }
  )

  // cw20 tokens
  const cw20CodeId = await uploadContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm"))

  const cw20Token1 = await instantiateContract(terra, deployer, cw20CodeId,
    {
      name: "cw20 Token 1",
      symbol: "ONE",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress }
    }
  )

  const cw20Token2 = await instantiateContract(terra, deployer, cw20CodeId,
    {
      name: "cw20 Token 2",
      symbol: "TWO",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress }
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
          max_loan_to_value: String(MAX_LTV),
          reserve_factor: "0.2",
          liquidation_threshold: String(MAX_LTV + 0.001),
          liquidation_bonus: String(LIQUIDATION_BONUS),
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
    },
    { logger: logger }
  )
  await setAssetOraclePriceSource(terra, deployer, oracle,
    { native: { denom: "uluna" } },
    LUNA_USD_PRICE,
    logger
  )
  const maUluna = await queryMaAssetAddress(terra, redBank, { native: { denom: "uluna" } })

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
          liquidation_bonus: String(LIQUIDATION_BONUS),
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
    },
    { logger: logger }
  )
  await setAssetOraclePriceSource(terra, deployer, oracle,
    { native: { denom: "uusd" } },
    1,
    logger
  )
  const maUusd = await queryMaAssetAddress(terra, redBank, { native: { denom: "uusd" } })

  // cw20token1
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { cw20: { contract_addr: cw20Token1 } },
        asset_params: {
          initial_borrow_rate: "0.1",
          max_loan_to_value: String(MAX_LTV),
          reserve_factor: "0.2",
          liquidation_threshold: String(MAX_LTV + 0.001),
          liquidation_bonus: String(LIQUIDATION_BONUS),
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
    },
    { logger: logger }
  )
  await setAssetOraclePriceSource(terra, deployer, oracle,
    { cw20: { contract_addr: cw20Token1 } },
    CW20_TOKEN_USD_PRICE,
    logger
  )
  const maCw20Token1 = await queryMaAssetAddress(terra, redBank, { cw20: { contract_addr: cw20Token1 } })

  // cw20token2
  await executeContract(terra, deployer, redBank,
    {
      init_asset: {
        asset: { cw20: { contract_addr: cw20Token2 } },
        asset_params: {
          initial_borrow_rate: "0.1",
          max_loan_to_value: String(MAX_LTV),
          reserve_factor: "0.2",
          liquidation_threshold: String(MAX_LTV + 0.001),
          liquidation_bonus: String(LIQUIDATION_BONUS),
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
    },
    { logger: logger }
  )
  await setAssetOraclePriceSource(terra, deployer, oracle,
    { cw20: { contract_addr: cw20Token2 } },
    CW20_TOKEN_USD_PRICE,
    logger
  )
  const maCw20Token2 = await queryMaAssetAddress(terra, redBank, { cw20: { contract_addr: cw20Token2 } })

  // astroport pair

  let result = await executeContract(terra, deployer, astroportFactory,
    {
      create_pair: {
        pair_type: { xyk: {} },
        asset_infos: [
          { token: { contract_addr: cw20Token1 } },
          { native_token: { denom: "uusd" } }
        ]
      }
    },
    { logger: logger }
  )
  const cw20Token1UusdPair = result.logs[0].eventsByType.wasm.pair_contract_addr[0]

  await mintCw20(terra, deployer, cw20Token1, deployer.key.accAddress, CW20_TOKEN_1_UUSD_PAIR_CW20_TOKEN_1_LP_AMOUNT, logger)

  await executeContract(terra, deployer, cw20Token1,
    {
      increase_allowance: {
        spender: cw20Token1UusdPair,
        amount: String(CW20_TOKEN_1_UUSD_PAIR_CW20_TOKEN_1_LP_AMOUNT),
      }
    },
    { logger: logger }
  )

  await executeContract(terra, deployer, cw20Token1UusdPair,
    {
      provide_liquidity: {
        assets: [
          {
            info: { token: { contract_addr: cw20Token1 } },
            amount: String(CW20_TOKEN_1_UUSD_PAIR_CW20_TOKEN_1_LP_AMOUNT)
          }, {
            info: { native_token: { denom: "uusd" } },
            amount: String(CW20_TOKEN_1_UUSD_PAIR_UUSD_LP_AMOUNT)
          }
        ]
      }
    },
    { coins: `${CW20_TOKEN_1_UUSD_PAIR_UUSD_LP_AMOUNT}uusd`, logger: logger }
  )

  // tests

  const env: Env = {
    terra,
    deployer,
    provider,
    borrower,
    cw20Token1,
    cw20Token2,
    maUluna,
    maUusd,
    maCw20Token1,
    maCw20Token2,
    redBank,
    protocolRewardsCollector,
    treasury,
    safetyFund,
    staking,
  }

  console.log("testNative")
  env.provider = terra.wallets.test2
  env.borrower = terra.wallets.test3
  await testNative(env, logger)

  console.log("testCw20")
  env.provider = terra.wallets.test4
  env.borrower = terra.wallets.test5
  await testCw20(env, logger)

  console.log("testLiquidateNative")
  env.provider = terra.wallets.test6
  env.borrower = terra.wallets.test7
  await testLiquidateNative(env, logger)

  console.log("testLiquidateCw20")
  env.provider = terra.wallets.test8
  env.borrower = terra.wallets.test9
  await testLiquidateCw20(env, logger)

  console.log("OK")

  logger.showGasConsumption()
})()
