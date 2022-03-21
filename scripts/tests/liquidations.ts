import {
  Coin,
  LocalTerra, MnemonicKey,
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
  instantiateContract,
  Logger,
  queryContract,
  setTimeoutDuration,
  setGasAdjustment,
  sleep,
  toEncodedBinary,
  uploadContract,
} from "../helpers.js"
import {
  borrowCw20,
  borrowNative,
  computeTax,
  deductTax,
  depositCw20,
  depositNative,
  mintCw20,
  queryBalanceCw20,
  queryMaAssetAddress,
  queryBalanceNative,
  setAssetOraclePriceSource
} from "./test_helpers.js"

// CONSTS

// required environment variables
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!

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

// TYPES
interface Env {
  terra: LocalTerra,
  redBank: string,
  deployer: Wallet,
  maUluna: string,
  cw20Token1: string,
  cw20Token2: string,
  maCw20Token2: string,
}

// TESTS

async function testCollateralizedNativeLoan(
  env: Env,
  borrower: Wallet,
  borrowFraction: number,
  receiveMaToken: Boolean,
  logger?: Logger
) {
  console.log("testCollateralizedNativeLoan: borrowFraction:", borrowFraction, "receiveMaToken:", receiveMaToken)

  const { terra, redBank, deployer, maUluna } = env

  console.log("provider provides uusd")

  const provider = deployer
  await depositNative(terra, provider, redBank, "uusd", USD_COLLATERAL_AMOUNT, logger)

  console.log("borrower provides uluna")

  await depositNative(terra, borrower, redBank, "uluna", LUNA_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows a small amount of uusd")

  let totalUusdAmountBorrowed = 0

  let uusdAmountBorrowed = Math.floor(USD_BORROW_AMOUNT * 0.01)
  let txResult = await borrowNative(terra, borrower, redBank, "uusd", uusdAmountBorrowed, logger)
  let txEvents = txResult.logs[0].eventsByType

  // amount received after deducting Terra tax from the borrowed amount
  let uusdAmountReceivedFromBorrow = Coin.fromString(txEvents.coin_received.amount[0]).amount.toNumber()
  let expectedUusdAmountReceived = (await deductTax(terra, new Coin("uusd", uusdAmountBorrowed))).toNumber()
  strictEqual(uusdAmountReceivedFromBorrow, expectedUusdAmountReceived)

  totalUusdAmountBorrowed += uusdAmountBorrowed

  console.log("liquidator tries to liquidate the borrower")

  const liquidator = deployer

  let uusdAmountLiquidated = uusdAmountBorrowed
  // should fail because the borrower's health factor is > 1
  await assert.rejects(
    executeContract(terra, liquidator, redBank,
      {
        liquidate_native: {
          collateral_asset: { native: { denom: "uluna" } },
          debt_asset_denom: "uusd",
          user_address: borrower.key.accAddress,
          receive_ma_token: receiveMaToken,
        }
      },
      { coins: `${uusdAmountLiquidated}uusd`, logger: logger }
    ),
    (error: any) => {
      return error.response.data.message.includes(
        "User's health factor is not less than 1 and thus cannot be liquidated"
      )
    }
  )

  console.log("borrower borrows uusd up to the borrow limit of their uluna collateral")

  uusdAmountBorrowed = Math.floor(USD_BORROW_AMOUNT * 0.98)
  txResult = await borrowNative(terra, borrower, redBank, "uusd", uusdAmountBorrowed, logger)
  txEvents = txResult.logs[0].eventsByType

  const amountIdx = txEvents.coin_received.receiver.indexOf(borrower.key.accAddress)
  uusdAmountReceivedFromBorrow = Coin.fromString(txEvents.coin_received.amount[amountIdx]).amount.toNumber()
  expectedUusdAmountReceived = (await deductTax(terra, new Coin("uusd", uusdAmountBorrowed))).toNumber()
  strictEqual(uusdAmountReceivedFromBorrow, expectedUusdAmountReceived)

  totalUusdAmountBorrowed += uusdAmountBorrowed

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

  // get the liquidator's balances before they liquidate the borrower
  const uusdBalanceBefore = await queryBalanceNative(terra, liquidator.key.accAddress, "uusd")
  const ulunaBalanceBefore = await queryBalanceNative(terra, liquidator.key.accAddress, "uluna")
  const maUlunaBalanceBefore = await queryBalanceCw20(terra, liquidator.key.accAddress, maUluna)

  // liquidate the borrower
  uusdAmountLiquidated = Math.floor(totalUusdAmountBorrowed * borrowFraction)
  txResult = await executeContract(terra, liquidator, redBank,
    {
      liquidate_native: {
        collateral_asset: { native: { denom: "uluna" } },
        debt_asset_denom: "uusd",
        user_address: borrower.key.accAddress,
        receive_ma_token: receiveMaToken,
      }
    },
  { coins: `${uusdAmountLiquidated}uusd`, logger: logger }
  )
  txEvents = txResult.logs[0].eventsByType
  await sleep(100)
  const txInfo = await terra.tx.txInfo(txResult.txhash)

  // get the liquidator's balances after they have liquidated the borrower
  const uusdBalanceAfter = await queryBalanceNative(terra, liquidator.key.accAddress, "uusd")
  const ulunaBalanceAfter = await queryBalanceNative(terra, liquidator.key.accAddress, "uluna")
  const maUlunaBalanceAfter = await queryBalanceCw20(terra, liquidator.key.accAddress, maUluna)

  // the maximum fraction of debt that can be liquidated is `CLOSE_FACTOR`
  // Debt will be greater than amount borrowed at the time of liquidation
  // so when testing overpaying the debt we choose a fraction of the debt that is high enough
  // (has to be significantly greater than CLOSE_FACTOR) so that the amount repaid is higher than
  // the max repayable debt
  const liquidatorOverpays = borrowFraction > CLOSE_FACTOR
  const expectedLiquidatedDebtFraction = liquidatorOverpays ? CLOSE_FACTOR : borrowFraction

  // debt amount repaid
  // the actual amount of debt repaid by the liquidator:
  // if `liquidatorOverpays == true` then `debtAmountRepaid < uusdAmountLiquidated`
  const debtAmountRepaid = parseInt(txEvents.wasm.debt_amount_repaid[0])

  if (liquidatorOverpays) {
    // pay back the maximum amount of debt allowed to be repaid.
    // the exact amount of debt owed at any time t changes as interest accrues,
    // but we can know the lower bound
    const lowerBoundDebtAmountRepaid = Math.floor(totalUusdAmountBorrowed * expectedLiquidatedDebtFraction)
    // use intervals because the exact amount of debt owed at any time t changes as interest accrues
    assert(
      // check that the actual amount of debt repaid is greater than the expected amount,
      // due to the debt accruing interest
      debtAmountRepaid > lowerBoundDebtAmountRepaid &&
      // check that the actual amount of debt repaid is less than the debt after one year
      debtAmountRepaid < lowerBoundDebtAmountRepaid * (1 + INTEREST_RATE)
    )
  } else {
    // pay back less than the maximum repayable debt
    const expectedDebtAmountRepaid = Math.floor(totalUusdAmountBorrowed * expectedLiquidatedDebtFraction)
    // check that the actual amount of debt repaid is equal to the expected amount of debt repaid
    strictEqual(debtAmountRepaid, expectedDebtAmountRepaid)
  }

  // refund amount
  const refundAmount = parseInt(txEvents.wasm.refund_amount[0])
  if (liquidatorOverpays) {
    // liquidator paid more than the maximum repayable debt, so is refunded the difference
    const expectedRefundAmount = uusdAmountLiquidated - debtAmountRepaid
    strictEqual(refundAmount, expectedRefundAmount)
  } else {
    // liquidator paid less than the maximum repayable debt, so no refund is owed
    strictEqual(refundAmount, 0)
  }

  // liquidator uusd balance
  const uusdBalanceDifference = uusdBalanceBefore - uusdBalanceAfter
  const uusdAmountLiquidatedTax = (await terra.utils.calculateTax(new Coin("uusd", uusdAmountLiquidated))).amount.toNumber()
  if (liquidatorOverpays) {
    const refundAmountTax = (await computeTax(terra, new Coin("uusd", refundAmount))).toNumber()
    const expectedUusdBalanceDifference = debtAmountRepaid + uusdAmountLiquidatedTax + refundAmountTax
    // TODO why is uusdBalanceDifference sometimes 1 or 2 uusd different from expected?
    // strictEqual(uusdBalanceDifference, expectedUusdBalanceDifference)
    // Check a tight interval instead of equality
    assert(Math.abs(uusdBalanceDifference - expectedUusdBalanceDifference) < 2)
  } else {
    const expectedUusdBalanceDifference = debtAmountRepaid + uusdAmountLiquidatedTax
    strictEqual(uusdBalanceDifference, expectedUusdBalanceDifference)
  }

  // collateral amount liquidated
  const collateralAmountLiquidated = parseInt(txEvents.wasm.collateral_amount_liquidated[0])
  const expectedCollateralAmountLiquidated =
    Math.floor(debtAmountRepaid * (1 + LIQUIDATION_BONUS) / LUNA_USD_PRICE)
  strictEqual(collateralAmountLiquidated, expectedCollateralAmountLiquidated)

  // collateral amount received
  if (receiveMaToken) {
    const maUlunaBalanceDifference = maUlunaBalanceAfter - maUlunaBalanceBefore
    strictEqual(maUlunaBalanceDifference, collateralAmountLiquidated * MA_TOKEN_SCALING_FACTOR)
  } else {
    const ulunaBalanceDifference = ulunaBalanceAfter - ulunaBalanceBefore
    const ulunaTxFee = txInfo.tx.auth_info.fee.amount.get("uluna")!.amount.toNumber()
    strictEqual(ulunaBalanceDifference, collateralAmountLiquidated - ulunaTxFee)
  }
}

async function testCollateralizedCw20Loan(
  env: Env,
  borrower: Wallet,
  borrowFraction: number,
  receiveMaToken: Boolean,
  logger?: Logger
) {
  console.log("testCollateralizedCw20Loan: borrowFraction:", borrowFraction, "receiveMaToken:", receiveMaToken)

  const { terra, redBank, deployer, cw20Token1, cw20Token2, maCw20Token2 } = env

  const provider = deployer
  const liquidator = deployer

  // mint some tokens
  await mintCw20(terra, deployer, cw20Token1, provider.key.accAddress, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)
  await mintCw20(terra, deployer, cw20Token2, borrower.key.accAddress, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)
  await mintCw20(terra, deployer, cw20Token1, liquidator.key.accAddress, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)

  console.log("provider provides cw20 token 1")

  await depositCw20(terra, provider, redBank, cw20Token1, CW20_TOKEN_1_COLLATERAL_AMOUNT, logger)

  console.log("borrower provides cw20 token 2")

  await depositCw20(terra, borrower, redBank, cw20Token2, CW20_TOKEN_2_COLLATERAL_AMOUNT, logger)

  console.log("borrower borrows a small amount of cw20 token 1")

  let totalCw20Token1AmountBorrowed = 0

  let cw20Token1AmountBorrowed = Math.floor(CW20_TOKEN_1_BORROW_AMOUNT * 0.01)
  let txResult = await borrowCw20(terra, borrower, redBank, cw20Token1, cw20Token1AmountBorrowed, logger)
  let txEvents = txResult.logs[0].eventsByType

  let amountIdx = txEvents.from_contract.action.indexOf('transfer')
  let cw20Token1AmountReceivedFromBorrow = parseInt(txEvents.from_contract.amount[amountIdx])
  let expectedCw20Token1AmountReceived = cw20Token1AmountBorrowed
  strictEqual(cw20Token1AmountReceivedFromBorrow, expectedCw20Token1AmountReceived)

  totalCw20Token1AmountBorrowed += cw20Token1AmountBorrowed

  console.log("liquidator tries to liquidate the borrower")

  let cw20Token1AmountLiquidated = cw20Token1AmountBorrowed
  // should fail because the borrower's health factor is > 1
  await assert.rejects(
    executeContract(terra, liquidator, cw20Token1,
      {
        send: {
          contract: redBank,
          amount: String(cw20Token1AmountLiquidated),
          msg: toEncodedBinary({
            liquidate_cw20: {
              collateral_asset: { cw20: { contract_addr: cw20Token2 } },
              user_address: borrower.key.accAddress,
              receive_ma_token: receiveMaToken,
            }
          })
        }
      },
      { logger: logger }
    ),
    (error: any) => {
      return error.response.data.message.includes(
        "User's health factor is not less than 1 and thus cannot be liquidated"
      )
    }
  )

  console.log("borrower borrows cw20 token 1 up to the borrow limit of their cw20 token 2 collateral")

  cw20Token1AmountBorrowed = Math.floor(CW20_TOKEN_1_BORROW_AMOUNT * 0.98)
  txResult = await borrowCw20(terra, borrower, redBank, cw20Token1, cw20Token1AmountBorrowed, logger)
  txEvents = txResult.logs[0].eventsByType

  amountIdx = txEvents.from_contract.action.indexOf('transfer')
  cw20Token1AmountReceivedFromBorrow = parseInt(txEvents.from_contract.amount[amountIdx])
  expectedCw20Token1AmountReceived = cw20Token1AmountBorrowed
  strictEqual(cw20Token1AmountReceivedFromBorrow, expectedCw20Token1AmountReceived)

  totalCw20Token1AmountBorrowed += cw20Token1AmountBorrowed

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

  // get the liquidator's balances before they liquidate the borrower
  const cw20Token1BalanceBefore = await queryBalanceCw20(terra, liquidator.key.accAddress, cw20Token1)
  const cw20Token2BalanceBefore = await queryBalanceCw20(terra, liquidator.key.accAddress, cw20Token2)
  const maCw20Token2BalanceBefore = await queryBalanceCw20(terra, liquidator.key.accAddress, maCw20Token2)

  // liquidate the borrower
  cw20Token1AmountLiquidated = Math.floor(totalCw20Token1AmountBorrowed * borrowFraction)
  txResult = await executeContract(terra, liquidator, cw20Token1,
    {
      send: {
        contract: redBank,
        amount: String(cw20Token1AmountLiquidated),
        msg: toEncodedBinary({
          liquidate_cw20: {
            collateral_asset: { cw20: { contract_addr: cw20Token2 } },
            user_address: borrower.key.accAddress,
            receive_ma_token: receiveMaToken,
          }
        })
      }
    },
    { logger: logger }
  )
  txEvents = txResult.logs[0].eventsByType

  // get the liquidator's balances after they have liquidated the borrower
  const cw20Token1BalanceAfter = await queryBalanceCw20(terra, liquidator.key.accAddress, cw20Token1)
  const cw20Token2BalanceAfter = await queryBalanceCw20(terra, liquidator.key.accAddress, cw20Token2)
  const maCw20Token2BalanceAfter = await queryBalanceCw20(terra, liquidator.key.accAddress, maCw20Token2)

  // the maximum fraction of debt that can be liquidated is `CLOSE_FACTOR`
  const expectedLiquidatedDebtFraction = borrowFraction > CLOSE_FACTOR ? CLOSE_FACTOR : borrowFraction

  // debt amount repaid
  const debtAmountRepaid = parseInt(txEvents.wasm.debt_amount_repaid[0])
  const expectedDebtAmountRepaid = Math.floor(totalCw20Token1AmountBorrowed * expectedLiquidatedDebtFraction)

  if (borrowFraction > CLOSE_FACTOR) {
    // pay back the maximum repayable debt
    // use intervals because the exact amount of debt owed at any time t changes as interest accrues
    assert(
      // check that the actual amount of debt repaid is greater than the expected amount,
      // due to the debt accruing interest
      debtAmountRepaid > expectedDebtAmountRepaid &&
      // check that the actual amount of debt repaid is less than the debt after one year
      debtAmountRepaid < expectedDebtAmountRepaid * (1 + INTEREST_RATE)
    )
  } else {
    // pay back less than the maximum repayable debt
    // check that the actual amount of debt repaid is equal to the expected amount of debt repaid
    strictEqual(debtAmountRepaid, expectedDebtAmountRepaid)
  }

  // liquidator cw20 token 1 balance
  const cw20Token1BalanceDifference = cw20Token1BalanceBefore - cw20Token1BalanceAfter
  strictEqual(cw20Token1BalanceDifference, debtAmountRepaid)

  // refund amount
  const refundAmount = parseInt(txEvents.wasm.refund_amount[0])
  if (borrowFraction > CLOSE_FACTOR) {
    // liquidator paid more than the maximum repayable debt, so is refunded the difference
    const expectedRefundAmount = cw20Token1AmountLiquidated - debtAmountRepaid
    strictEqual(refundAmount, expectedRefundAmount)
  } else {
    // liquidator paid less than the maximum repayable debt, so no refund is owed
    strictEqual(refundAmount, 0)
  }

  // collateral amount liquidated
  const collateralAmountLiquidated = parseInt(txEvents.wasm.collateral_amount_liquidated[0])
  const expectedCollateralAmountLiquidated = Math.floor(debtAmountRepaid * (1 + LIQUIDATION_BONUS))
  strictEqual(collateralAmountLiquidated, expectedCollateralAmountLiquidated)

  // collateral amount received
  if (receiveMaToken) {
    const maCw20Token2BalanceDifference = maCw20Token2BalanceAfter - maCw20Token2BalanceBefore
    strictEqual(maCw20Token2BalanceDifference, collateralAmountLiquidated * MA_TOKEN_SCALING_FACTOR)
  } else {
    const cw20Token2BalanceDifference = cw20Token2BalanceAfter - cw20Token2BalanceBefore
    strictEqual(cw20Token2BalanceDifference, collateralAmountLiquidated)
  }
}

async function testUncollateralizedNativeLoan(
  env: Env,
  borrower: Wallet,
  logger?: Logger
) {
  console.log("testUncollateralizedNativeLoan")

  const { terra, redBank, deployer } = env

  console.log("provider provides uusd")

  const provider = deployer

  await depositNative(terra, provider, redBank, "uusd", USD_COLLATERAL_AMOUNT, logger)

  console.log("set uncollateralized loan limit for borrower")

  await executeContract(terra, deployer, redBank,
    {
      update_uncollateralized_loan_limit: {
        user_address: borrower.key.accAddress,
        asset: { native: { denom: "uusd" } },
        new_limit: String(USD_COLLATERAL_AMOUNT),
      }
    },
    { logger: logger }
  )

  console.log("borrower borrows uusd")

  const uusdBalanceBefore = await queryBalanceNative(terra, borrower.key.accAddress, "uusd")

  const uusdAmountBorrowed = USD_COLLATERAL_AMOUNT
  let txResult = await borrowNative(terra, borrower, redBank, "uusd", uusdAmountBorrowed, logger)
  const txEvents = txResult.logs[0].eventsByType
  const loggedUusdAmountBorrowed = parseInt(txEvents.wasm.amount[0])
  strictEqual(loggedUusdAmountBorrowed, uusdAmountBorrowed)

  const uusdBalanceAfter = await queryBalanceNative(terra, borrower.key.accAddress, "uusd")
  const uusdBalanceDifference = uusdBalanceAfter - uusdBalanceBefore
  const uusdAmountBorrowedTax = (await computeTax(terra, new Coin("uusd", uusdAmountBorrowed))).toNumber()
  strictEqual(uusdBalanceDifference, uusdAmountBorrowed - uusdAmountBorrowedTax)

  console.log("liquidator tries to liquidate the borrower")

  const liquidator = deployer

  // check user position
  const userPositionT1 = await queryContract(terra, redBank,
    { user_position: { user_address: borrower.key.accAddress } }
  )
  strictEqual(userPositionT1.health_status, "not_borrowing")


  // should fail because there are no collateralized loans
  await assert.rejects(
    executeContract(terra, liquidator, redBank,
      {
        liquidate_native: {
          collateral_asset: { native: { denom: "uluna" } },
          debt_asset_denom: "uusd",
          user_address: borrower.key.accAddress,
          receive_ma_token: false,
        }
      },

      { coins: `${uusdAmountBorrowed}uusd`, logger: logger }
    ),
    (error: any) => {
      return error.response.data.message.includes(
        "User has a positive uncollateralized loan limit and thus cannot be liquidated"
      )
    }
  )

  const userPositionT2 = await queryContract(terra, redBank,
    { user_position: { user_address: borrower.key.accAddress } }
  )
  strictEqual(userPositionT1.total_collateralized_debt_in_uusd, userPositionT2.total_collateralized_debt_in_uusd)
  strictEqual(userPositionT1.max_debt_in_uusd, userPositionT2.max_debt_in_uusd)
}

// MAIN

(async () => {
  setTimeoutDuration(0)
  setGasAdjustment(2)

  const logger = new Logger()

  const terra = new LocalTerra()
  const deployer = terra.wallets.test1
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
        safety_fund_fee_share: "0.1",
        treasury_fee_share: "0.2",
        ma_token_code_id: maTokenCodeId,
        close_factor: String(CLOSE_FACTOR),
      }
    }
  )

  // update address provider
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

  const env: Env = { terra, redBank, deployer, maUluna, cw20Token1, cw20Token2, maCw20Token2 }

  // collateralized
  let borrowFraction = CLOSE_FACTOR - 0.1
  await testCollateralizedNativeLoan(env, terra.wallets.test2, borrowFraction, false, logger)
  await testCollateralizedNativeLoan(env, terra.wallets.test3, borrowFraction, true, logger)
  await testCollateralizedCw20Loan(env, terra.wallets.test4, borrowFraction, false, logger)
  await testCollateralizedCw20Loan(env, terra.wallets.test5, borrowFraction, true, logger)

  borrowFraction = CLOSE_FACTOR + 0.1
  await testCollateralizedNativeLoan(env, terra.wallets.test6, borrowFraction, false, logger)
  await testCollateralizedNativeLoan(env, terra.wallets.test7, borrowFraction, true, logger)
  await testCollateralizedCw20Loan(env, terra.wallets.test8, borrowFraction, false, logger)
  await testCollateralizedCw20Loan(env, terra.wallets.test9, borrowFraction, true, logger)

  // uncollateralized
  await testUncollateralizedNativeLoan(env, terra.wallets.test10, logger)

  console.log("OK")

  logger.showGasConsumption()
})()
