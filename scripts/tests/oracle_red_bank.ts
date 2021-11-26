/*
LocalTerra requires >= 1500 ms block times for the native Terra oracle to work:

```
sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/1500ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
```
*/

import { LCDClient, LocalTerra } from "@terra-money/terra.js"
import { strictEqual } from "assert"
import { join } from "path"
import 'dotenv/config.js'
import {
  deployContract,
  executeContract,
  queryContract,
  setTimeoutDuration,
  sleep,
  uploadContract
} from "../helpers.js"
import {
  approximateEqual,
  depositNative,
} from "./test_helpers.js"

// CONSTS

// required environment variables:
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH!

// HELPERS

async function waitUntilTerraOracleAvailable(terra: LCDClient) {
  let tries = 0
  const maxTries = 10
  let backoff = 1
  while (true) {
    const activeDenoms = await terra.oracle.activeDenoms()
    if (activeDenoms.includes("uusd")) {
      break
    }

    // timeout
    tries++
    if (tries == maxTries) {
      throw new Error(`Terra oracle not available after ${maxTries} tries`)
    }

    // exponential backoff
    console.log(`Terra oracle not available, sleeping for ${backoff} s`)
    await sleep(backoff * 1000)
    backoff *= 2
  }
}

// MAIN

(async () => {
  setTimeoutDuration(0)

  const terra = new LocalTerra()

  await waitUntilTerraOracleAvailable(terra)

  // addresses
  const deployer = terra.wallets.test1
  // mock contract addresses
  const astroportGenerator = terra.wallets.test9.key.accAddress
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
          protocol_rewards_collector_address: protocolRewardsCollector,
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
          initial_borrow_rate: "0.2",
          max_loan_to_value: "0.75",
          reserve_factor: "0.2",
          liquidation_threshold: "0.85",
          liquidation_bonus: "0.1",
          interest_rate_strategy: {
            dynamic: {
              min_borrow_rate: "0.0",
              max_borrow_rate: "1.0",
              kp_1: "0.04",
              optimal_utilization_rate: "0.9",
              kp_augmentation_threshold: "0.15",
              kp_2: "0.07"
            }
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true
        }
      }
    }
  )

  console.log("setup astroport pair")

  const tokenCodeID = await uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_token.wasm"))
  const pairCodeID = await uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_pair.wasm"))
  const astroportFactory = await deployContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, "astroport_factory.wasm"),
    {
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

  let result = await executeContract(terra, deployer, astroportFactory,
    {
      create_pair: {
        pair_type: { xyk: {} },
        asset_infos: [
          { native_token: { denom: "uluna" } },
          { native_token: { denom: "uusd" } }
        ]
      }
    }
  )
  const ulunaUusdPair = result.logs[0].eventsByType.wasm.pair_contract_addr[0]

  // TESTS

  console.log("test oracle price sources")

  {
    console.log("- fixed")

    await executeContract(terra, deployer, oracle,
      {
        set_asset: {
          asset: { native: { denom: "uluna" } },
          price_source: { fixed: { price: "25" } }
        }
      }
    )

    const alice = terra.wallets.test2

    await depositNative(terra, alice, redBank, "uluna", 1_000000)

    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: alice.key.accAddress } }
    )

    // 1 luna should be worth $25
    strictEqual(parseInt(userPosition.total_collateral_in_uusd), 25_000000)
  }

  {
    console.log("- astroport spot")

    await executeContract(terra, deployer, oracle,
      {
        set_asset: {
          asset: { native: { denom: "uluna" } },
          price_source: { astroport_spot: { pair_address: ulunaUusdPair } }
        }
      }
    )

    const bob = terra.wallets.test3

    await depositNative(terra, bob, redBank, "uluna", 1_000000)

    // provide liquidity such that the price of luna is $30
    await executeContract(terra, deployer, ulunaUusdPair,
      {
        provide_liquidity: {
          assets: [
            {
              info: { native_token: { denom: "uluna" } },
              amount: String(1_000_000_000000)
            }, {
              info: { native_token: { denom: "uusd" } },
              amount: String(30_000_000_000000),
            }
          ]
        }
      },
      `1000000000000uluna,30000000000000uusd`,
    )

    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: bob.key.accAddress } }
    )

    // 1 luna should be worth $30
    approximateEqual(parseInt(userPosition.total_collateral_in_uusd), 30_000000, 100)
  }

  {
    console.log("- astroport twap")

    await executeContract(terra, deployer, oracle,
      {
        set_asset: {
          asset: { native: { denom: "uluna" } },
          price_source: {
            astroport_twap: {
              pair_address: ulunaUusdPair,
              window_size: 2,
              tolerance: 1,
            }
          }
        }
      }
    )

    const carol = terra.wallets.test4

    await depositNative(terra, carol, redBank, "uluna", 1_000000)

    // trigger cumulative prices to be updated
    await executeContract(terra, deployer, ulunaUusdPair,
      {
        provide_liquidity: {
          assets: [
            {
              info: { native_token: { denom: "uluna" } },
              amount: String(1)
            }, {
              info: { native_token: { denom: "uusd" } },
              amount: String(30),
            }
          ]
        }
      },
      `1uluna,30uusd`,
    )

    // record TWAP
    await executeContract(terra, deployer, oracle,
      { record_twap_snapshots: { assets: [{ native: { denom: "uluna" } }] } }
    )

    // wait until a twap snapshot can be recorded again
    await sleep(1500)

    // record TWAP
    await executeContract(terra, deployer, oracle,
      { record_twap_snapshots: { assets: [{ native: { denom: "uluna" } }] } }
    )

    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: carol.key.accAddress } }
    )

    // 1 luna should be worth $30
    strictEqual(parseInt(userPosition.total_collateral_in_uusd), 30_000000)
  }

  {
    console.log("- native")

    await executeContract(terra, deployer, oracle,
      {
        set_asset: {
          asset: { native: { denom: "uluna" } },
          price_source: { native: { denom: "uluna" } }
        }
      }
    )

    const dan = terra.wallets.test5

    await depositNative(terra, dan, redBank, "uluna", 1_000000)

    const userPosition = await queryContract(terra, redBank,
      { user_position: { user_address: dan.key.accAddress } }
    )

    const lunaUsdPrice = await terra.oracle.exchangeRate("uusd")
    const lunaUusdPrice = lunaUsdPrice?.amount.mul(1_000000).floor().toNumber()
    strictEqual(parseInt(userPosition.total_collateral_in_uusd), lunaUusdPrice)
  }

  console.log("OK")
})()
