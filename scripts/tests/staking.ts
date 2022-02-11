import {
  LCDClient,
  LocalTerra,
  MnemonicKey,
  MsgSend
} from "@terra-money/terra.js"
import {
  strictEqual,
  strict as assert
} from "assert"
import { join } from "path"
import 'dotenv/config.js'
import {
  deployContract,
  executeContract, Logger,
  performTransaction,
  queryContract,
  setGasAdjustment,
  setTimeoutDuration,
  sleep,
  toEncodedBinary,
  uploadContract
} from "../helpers.js"
import {
  approximateEqual,
  getBlockHeight,
  mintCw20,
  queryBalanceCw20,
  queryBalanceNative,
  transferCw20
} from "./test_helpers.js"

// CONSTS

// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH!

const COOLDOWN_DURATION_SECONDS = 2
const MARS_STAKE_AMOUNT = 1_000_000000
const UUSD_REWARDS_AMOUNT = 100_000000

const LUNA_USD_PRICE = 25
const ULUNA_UUSD_PAIR_ULUNA_LP_AMOUNT = 1_000_000_000000
const ULUNA_UUSD_PAIR_UUSD_LP_AMOUNT = ULUNA_UUSD_PAIR_ULUNA_LP_AMOUNT * LUNA_USD_PRICE
const MARS_USD_PRICE = 2
const MARS_UUSD_PAIR_MARS_LP_AMOUNT = 1_000_000_000000
const MARS_UUSD_PAIR_UUSD_LP_AMOUNT = MARS_UUSD_PAIR_MARS_LP_AMOUNT * MARS_USD_PRICE

// HELPERS

async function assertXmarsBalance(
  terra: LCDClient,
  xMars: string,
  address: string,
  expectedBalance: number,
) {
  const balance = await queryBalanceCw20(terra, address, xMars)
  strictEqual(balance, expectedBalance)
}

async function assertXmarsBalanceAt(
  terra: LCDClient,
  xMars: string,
  address: string,
  block: number,
  expectedBalance: number,
) {
  const xMarsBalance = await queryContract(terra, xMars, { balance_at: { address, block } })
  strictEqual(parseInt(xMarsBalance.balance), expectedBalance)
}

async function assertXmarsTotalSupplyAt(
  terra: LCDClient,
  xMars: string,
  block: number,
  expectedTotalSupply: number,
) {
  const expectedXmarsTotalSupply = await queryContract(terra, xMars, { total_supply_at: { block } })
  strictEqual(parseInt(expectedXmarsTotalSupply.total_supply), expectedTotalSupply)
}

// MAIN

(async () => {
  setTimeoutDuration(0)
  setGasAdjustment(2)

  const logger = new Logger()

  const terra = new LocalTerra()

  // addresses
  const deployer = terra.wallets.test1
  const alice = terra.wallets.test2
  const bob = terra.wallets.test3
  const carol = terra.wallets.test4
  const dan = terra.wallets.test5
  // mock contract addresses
  const astroportGenerator = new MnemonicKey().accAddress

  console.log("upload contracts")

  const addressProvider = await deployContract(terra, deployer, "../artifacts/mars_address_provider.wasm",
    { owner: deployer.key.accAddress }
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

  const staking = await deployContract(terra, deployer, "../artifacts/mars_staking.wasm",
    {
      config: {
        owner: deployer.key.accAddress,
        address_provider_address: addressProvider,
        astroport_factory_address: astroportFactory,
        astroport_max_spread: "0.05",
        cooldown_duration: COOLDOWN_DURATION_SECONDS,
      }
    }
  )

  const mars = await deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm"),
    {
      name: "Mars",
      symbol: "MARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress },
    }
  )

  const xMars = await deployContract(terra, deployer, "../artifacts/mars_xmars_token.wasm",
    {
      name: "xMars",
      symbol: "xMARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: staking },
    }
  )

  // update address provider
  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          mars_token_address: mars,
          staking_address: staking,
          xmars_token_address: xMars,
          protocol_admin_address: deployer.key.accAddress,
        }
      }
    },
    { logger: logger }
  )

  // astroport pairs

  let result = await executeContract(terra, deployer, astroportFactory,
    {
      create_pair: {
        pair_type: { xyk: {} },
        asset_infos: [
          { token: { contract_addr: mars } },
          { native_token: { denom: "uusd" } }
        ]
      }
    },
    { logger: logger }
  )
  const marsUusdPair = result.logs[0].eventsByType.wasm.pair_contract_addr[0]

  result = await executeContract(terra, deployer, astroportFactory,
    {
      create_pair: {
        pair_type: { xyk: {} },
        asset_infos: [
          { native_token: { denom: "uluna" } },
          { native_token: { denom: "uusd" } }
        ]
      }
    },
    { logger: logger }
  )
  const ulunaUusdPair = result.logs[0].eventsByType.wasm.pair_contract_addr[0]

  await executeContract(terra, deployer, ulunaUusdPair,
    {
      provide_liquidity: {
        assets: [
          {
            info: { native_token: { denom: "uluna" } },
            amount: String(ULUNA_UUSD_PAIR_ULUNA_LP_AMOUNT)
          }, {
            info: { native_token: { denom: "uusd" } },
            amount: String(ULUNA_UUSD_PAIR_UUSD_LP_AMOUNT)
          }
        ]
      }
    },
    { coins: `${ULUNA_UUSD_PAIR_ULUNA_LP_AMOUNT}uluna,${ULUNA_UUSD_PAIR_UUSD_LP_AMOUNT}uusd`, logger: logger }
  )

  await mintCw20(terra, deployer, mars, deployer.key.accAddress, MARS_UUSD_PAIR_MARS_LP_AMOUNT, logger)

  await executeContract(terra, deployer, mars,
    {
      increase_allowance: {
        spender: marsUusdPair,
        amount: String(MARS_UUSD_PAIR_MARS_LP_AMOUNT),
      }
    },
    { logger: logger }
  )

  await executeContract(terra, deployer, marsUusdPair,
    {
      provide_liquidity: {
        assets: [
          {
            info: { token: { contract_addr: mars } },
            amount: String(MARS_UUSD_PAIR_MARS_LP_AMOUNT)
          }, {
            info: { native_token: { denom: "uusd" } },
            amount: String(MARS_UUSD_PAIR_UUSD_LP_AMOUNT)
          }
        ]
      }
    },
    { coins: `${MARS_UUSD_PAIR_UUSD_LP_AMOUNT}uusd`, logger: logger }
  )

  // TESTS

  let expectedXmarsTotalSupply = 0

  {
    console.log("alice stakes Mars and receives the same amount of xMars")

    await mintCw20(terra, deployer, mars, alice.key.accAddress, MARS_STAKE_AMOUNT, logger)

    const txResult = await executeContract(terra, alice, mars,
      {
        send: {
          contract: staking,
          amount: String(MARS_STAKE_AMOUNT),
          msg: toEncodedBinary({ stake: {} })
        }
      },
      { logger: logger }
    )
    const block = await getBlockHeight(terra, txResult)

    // before staking
    await assertXmarsBalanceAt(terra, xMars, alice.key.accAddress, block - 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after staking
    expectedXmarsTotalSupply += MARS_STAKE_AMOUNT
    await assertXmarsBalance(terra, xMars, alice.key.accAddress, MARS_STAKE_AMOUNT)
    await assertXmarsBalanceAt(terra, xMars, alice.key.accAddress, block + 1, MARS_STAKE_AMOUNT)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  {
    console.log("bob stakes Mars and receives the same amount of xMars")

    await mintCw20(terra, deployer, mars, bob.key.accAddress, MARS_STAKE_AMOUNT, logger)

    const txResult = await executeContract(terra, bob, mars,
      {
        send: {
          contract: staking,
          amount: String(MARS_STAKE_AMOUNT),
          msg: toEncodedBinary({ stake: {} })
        }
      },
      { logger: logger }
    )
    const block = await getBlockHeight(terra, txResult)

    // before staking
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block - 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after staking
    expectedXmarsTotalSupply += MARS_STAKE_AMOUNT
    await assertXmarsBalance(terra, xMars, bob.key.accAddress, MARS_STAKE_AMOUNT)
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block + 1, MARS_STAKE_AMOUNT)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  {
    console.log("bob transfers half of his xMars to alice")

    const txResult = await transferCw20(terra, bob, xMars, alice.key.accAddress, MARS_STAKE_AMOUNT / 2, logger)
    const block = await getBlockHeight(terra, txResult)

    // before staking
    await assertXmarsBalanceAt(terra, xMars, alice.key.accAddress, block - 1, MARS_STAKE_AMOUNT)
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block - 1, MARS_STAKE_AMOUNT)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after staking
    await assertXmarsBalance(terra, xMars, alice.key.accAddress, 3 * MARS_STAKE_AMOUNT / 2)
    await assertXmarsBalance(terra, xMars, bob.key.accAddress, MARS_STAKE_AMOUNT / 2)
    await assertXmarsBalanceAt(terra, xMars, alice.key.accAddress, block + 1, 3 * MARS_STAKE_AMOUNT / 2)
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block + 1, MARS_STAKE_AMOUNT / 2)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  {
    console.log("swap protocol rewards to USD, then USD to Mars")

    // send uusd to the staking contract to simulate rewards accrued to stakers sent form the rewards distributor 
    await performTransaction(terra, deployer,
      new MsgSend(deployer.key.accAddress, staking, { uusd: UUSD_REWARDS_AMOUNT })
    )

    // swap usd to mars
    const uusdBalanceBeforeSwapToMars = await queryBalanceNative(terra, staking, "uusd")
    const marsBalanceBeforeSwapToMars = await queryBalanceCw20(terra, staking, mars)

    // don't swap the entire uusd balance, otherwise there won't be enough to pay the tx fee
    const uusdSwapAmount = uusdBalanceBeforeSwapToMars - 10_000000

    await executeContract(terra, deployer, staking,
      { swap_uusd_to_mars: { amount: String(uusdSwapAmount) } }, { logger: logger }
    )

    const marsBalanceAfterSwapToMars = await queryBalanceCw20(terra, staking, mars)
    const uusdBalanceAfterSwapToMars = await queryBalanceNative(terra, staking, "uusd")

    assert(uusdBalanceAfterSwapToMars < uusdBalanceBeforeSwapToMars)
    assert(marsBalanceAfterSwapToMars > marsBalanceBeforeSwapToMars)
  }

  {
    console.log("carol stakes Mars and receives a smaller amount of xMars")

    await mintCw20(terra, deployer, mars, carol.key.accAddress, MARS_STAKE_AMOUNT, logger)

    const txResult = await executeContract(terra, carol, mars,
      {
        send: {
          contract: staking,
          amount: String(MARS_STAKE_AMOUNT),
          msg: toEncodedBinary({ stake: {} })
        }
      },
      { logger: logger }
    )
    const block = await getBlockHeight(terra, txResult)

    // before staking
    await assertXmarsBalanceAt(terra, xMars, carol.key.accAddress, block - 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after staking
    const carolXmarsBalance = await queryBalanceCw20(terra, carol.key.accAddress, xMars)
    assert(carolXmarsBalance < MARS_STAKE_AMOUNT)
    expectedXmarsTotalSupply += carolXmarsBalance
    await assertXmarsBalanceAt(terra, xMars, carol.key.accAddress, block + 1, carolXmarsBalance)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  let bobCooldownEnd: number

  {
    console.log("bob unstakes xMars")

    const bobXmarsBalance = await queryBalanceCw20(terra, bob.key.accAddress, xMars)
    const unstakeAmount = bobXmarsBalance

    const cooldownStart = Date.now()
    bobCooldownEnd = cooldownStart + COOLDOWN_DURATION_SECONDS * 1000 // ms

    const txResult = await executeContract(terra, bob, xMars,
      {
        send: {
          contract: staking,
          amount: String(unstakeAmount),
          msg: toEncodedBinary({ unstake: {} })
        }
      },
      { logger: logger }
    )
    const block = await getBlockHeight(terra, txResult)

    const claim = await queryContract(terra, staking, { claim: { user_address: bob.key.accAddress } })
    assert(parseInt(claim.claim.amount) > 0)

    // before unstaking
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block - 1, MARS_STAKE_AMOUNT / 2)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after unstaking
    expectedXmarsTotalSupply -= MARS_STAKE_AMOUNT / 2
    // check xMars is burnt
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block + 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)

    console.log("claiming before cooldown has ended fails")

    await assert.rejects(
      executeContract(terra, bob, staking, { claim: {} }, { logger: logger }),
      (error: any) => {
        return error.response.data.message.includes("Cooldown has not ended")
      }
    )
  }

  {
    console.log("check that claimed Mars is not used in the Mars/xMars exchange rate when dan stakes Mars")

    await mintCw20(terra, deployer, mars, dan.key.accAddress, MARS_STAKE_AMOUNT, logger)

    const stakingMarsBalance = await queryBalanceCw20(terra, staking, mars)
    const globalState = await queryContract(terra, staking, { global_state: {} })
    const totalMarsForClaimers = parseInt(globalState.total_mars_for_claimers)
    const totalMarsForStakers = stakingMarsBalance - totalMarsForClaimers

    const txResult = await executeContract(terra, dan, mars,
      {
        send: {
          contract: staking,
          amount: String(MARS_STAKE_AMOUNT),
          msg: toEncodedBinary({ stake: {} })
        }
      },
      { logger: logger }
    )
    const block = await getBlockHeight(terra, txResult)

    const expectedDanXmarsBalance = Math.floor(MARS_STAKE_AMOUNT * (expectedXmarsTotalSupply / totalMarsForStakers))
    const danXmarsBalance = await queryBalanceCw20(terra, dan.key.accAddress, xMars)
    strictEqual(danXmarsBalance, expectedDanXmarsBalance)
    assert(danXmarsBalance < MARS_STAKE_AMOUNT)

    // before staking
    await assertXmarsBalanceAt(terra, xMars, dan.key.accAddress, block - 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)

    // after staking
    expectedXmarsTotalSupply += danXmarsBalance
    await assertXmarsBalanceAt(terra, xMars, dan.key.accAddress, block + 1, danXmarsBalance)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  {
    console.log("bob claims the amount of Mars he unstaked")

    const cooldownRemaining = Math.max(bobCooldownEnd - Date.now(), 0)
    await sleep(cooldownRemaining)

    const claim = await queryContract(terra, staking, { claim: { user_address: bob.key.accAddress } })

    const bobMarsBalanceBefore = await queryBalanceCw20(terra, bob.key.accAddress, mars)

    const txResult = await executeContract(terra, bob, staking, { claim: {} }, { logger: logger })
    const block = await getBlockHeight(terra, txResult)

    const bobMarsBalanceAfter = await queryBalanceCw20(terra, bob.key.accAddress, mars)
    strictEqual(parseInt(claim.claim.amount), bobMarsBalanceAfter - bobMarsBalanceBefore)

    // before and after claiming are the same
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block - 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block - 1, expectedXmarsTotalSupply)
    await assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, block + 1, 0)
    await assertXmarsTotalSupplyAt(terra, xMars, block + 1, expectedXmarsTotalSupply)
  }

  {
    console.log("carol unstakes xMars")

    const carolXmarsBalance = await queryBalanceCw20(terra, carol.key.accAddress, xMars)
    const unstakeAmount = carolXmarsBalance

    await executeContract(terra, carol, xMars,
      {
        send: {
          contract: staking,
          amount: String(unstakeAmount),
          msg: toEncodedBinary({ unstake: {} })
        }
      },
      { logger: logger }
    )

    expectedXmarsTotalSupply -= unstakeAmount
  }

  let danClaimAmount: number

  {
    console.log("check that claimed Mars is not used in the Mars/xMars exchange rate when dan unstakes xMars")

    const stakingMarsBalance = await queryBalanceCw20(terra, staking, mars)
    const globalState = await queryContract(terra, staking, { global_state: {} })
    const totalMarsForClaimers = parseInt(globalState.total_mars_for_claimers)
    const totalMarsForStakers = stakingMarsBalance - totalMarsForClaimers

    const danXmarsBalance = await queryBalanceCw20(terra, dan.key.accAddress, xMars)
    const unstakeAmount = danXmarsBalance

    await executeContract(terra, dan, xMars,
      {
        send: {
          contract: staking,
          amount: String(unstakeAmount),
          msg: toEncodedBinary({ unstake: {} })
        }
      },
      { logger: logger }
    )

    const claim = await queryContract(terra, staking, { claim: { user_address: dan.key.accAddress } })
    danClaimAmount = parseInt(claim.claim.amount)
    const expectedDanMarsBalance = Math.floor(unstakeAmount * (totalMarsForStakers / expectedXmarsTotalSupply))
    strictEqual(danClaimAmount, expectedDanMarsBalance)
  }

  {
    console.log("slash stakers by transferring Mars from the staking contract")

    const stakingMarsBalanceBefore = await queryBalanceCw20(terra, staking, mars)
    const deployerMarsBalanceBefore = await queryBalanceCw20(terra, deployer.key.accAddress, mars)
    const marsForClaimersBefore = (await queryContract(terra, staking, { global_state: {} })).total_mars_for_claimers

    // slash 10% of the Mars balance
    const transferMarsAmount = Math.floor(stakingMarsBalanceBefore / 10)

    const txResult = await executeContract(terra, deployer, staking,
      {
        transfer_mars: {
          recipient: deployer.key.accAddress,
          amount: String(transferMarsAmount)
        }
      },
      { logger: logger }
    )

    const slashPercentage = parseFloat(txResult.logs[0].eventsByType.wasm.slash_percentage[0])
    approximateEqual(slashPercentage, 0.1, 0.0001)

    const stakingMarsBalanceAfter = await queryBalanceCw20(terra, staking, mars)
    const deployerMarsBalanceAfter = await queryBalanceCw20(terra, deployer.key.accAddress, mars)
    const marsForClaimersAfter = (await queryContract(terra, staking, { global_state: {} })).total_mars_for_claimers
    strictEqual(stakingMarsBalanceAfter, stakingMarsBalanceBefore - transferMarsAmount)
    strictEqual(deployerMarsBalanceAfter, deployerMarsBalanceBefore + transferMarsAmount)
    strictEqual(Math.floor(marsForClaimersBefore * 0.9), parseInt(marsForClaimersAfter))

  }

  {
    console.log("check that dan's claim has been slashed")

    const claim = await queryContract(terra, staking, { claim: { user_address: dan.key.accAddress } })
    const danClaimAmountAfterSlashing = parseInt(claim.claim.amount)
    approximateEqual(danClaimAmount * 0.9, danClaimAmountAfterSlashing, 1)
  }

  console.log("OK")

  logger.showGasConsumption()
})()
