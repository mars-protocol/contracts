import { LocalTerra, MnemonicKey } from "@terra-money/terra.js";
import { join } from "path";
import { strictEqual } from "assert";
import "dotenv/config.js";
import {
  deployContract,
  executeContract,
  instantiateContract, Logger,
  queryContract,
  setGasAdjustment,
  toEncodedBinary,
  uploadContract,
} from "../helpers.js";
import { queryBalanceCw20, queryBalanceNative } from "./test_helpers.js";

// TYPES

type Snapshot = {
  timestamp: number;
  cumulativePrice: number;
};

// CONSTS

// required environment variables:
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH!;

// terra LCD instance
const terra = new LocalTerra();

// accounts
const deployer = terra.wallets.test1;
const alice = terra.wallets.test2; // alice will provide initial liquidity to the astroport pair
const bob = terra.wallets.test3; // bob will trade in the pair, altering the price
const charlie = terra.wallets.test4; // charlies is a bot who calls the function to take TWAP snapshots

// contracts
let anchorToken: string;
let astroportFactory: string;
let astroportPair: string;
let astroportGenerator: string;
let astroportLiquidityToken: string;
let oracle: string;

// HELPERS

const diff = (a: number, b: number) => (a > b ? a - b : b - a);

async function expectPromiseToFail(promise: Promise<any>) {
  let failed = false;
  try {
    await promise;
  } catch {
    failed = true;
  }
  if (!failed) {
    throw new Error("expecting to fail but was successful?!");
  }
}

async function recordTwapSnapshots(logger?: Logger) {
  const result = await executeContract(terra, charlie, oracle, {
    record_twap_snapshots: {
      assets: [
        {
          cw20: {
            contract_addr: anchorToken,
          },
        },
      ],
    },
  }, { logger: logger });
  const timestamp = parseInt(result.logs[0].eventsByType.from_contract.timestamp[0]);
  const cumulativePrice = parseInt(result.logs[0].eventsByType.wasm.price_cumulative[0]);
  return { timestamp, cumulativePrice };
}

async function assertOraclePrice(token: string, expectedPrice: string) {
  const price: string = await queryContract(terra, oracle, {
    asset_price: {
      asset: {
        cw20: {
          contract_addr: token,
        },
      },
    },
  });
  strictEqual(price, expectedPrice);
}

// MAIN

(async () => {
  console.log("deployer:", deployer.key.accAddress);
  console.log("alice:   ", alice.key.accAddress);
  console.log("bob:     ", bob.key.accAddress);
  console.log("charlie: ", charlie.key.accAddress);

  const logger = new Logger()

  process.stdout.write("deploying anchor token... ");
  const cw20CodeId = await uploadContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_token.wasm")
  );
  anchorToken = await instantiateContract(terra, deployer, cw20CodeId, {
    name: "Anchor Token",
    symbol: "ANC",
    decimals: 6,
    initial_balances: [
      {
        address: alice.key.accAddress,
        amount: "10000000000",
      },
      {
        address: bob.key.accAddress,
        amount: "10000000000",
      },
    ],
  });
  console.log("success!");

  process.stdout.write("deploying astroport factory... ");
  const pairCodeId = await uploadContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_pair.wasm")
  );
  astroportGenerator = new MnemonicKey().accAddress
  astroportFactory = await deployContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_factory.wasm"),
    {
      owner: deployer.key.accAddress,
      token_code_id: cw20CodeId,
      generator_address: astroportGenerator,
      pair_configs: [
        {
          code_id: pairCodeId,
          pair_type: { xyk: {} },
          total_fee_bps: 30, // 30 bps = 0.3% swap fee
          maker_fee_bps: 0,
        },
      ],
    }
  );
  console.log("success!");

  process.stdout.write("creating astroport ANC-UST pair... ");
  const result1 = await executeContract(terra, deployer, astroportFactory, {
    create_pair: {
      pair_type: { xyk: {} },
      asset_infos: [
        {
          native_token: {
            denom: "uusd",
          },
        },
        {
          token: {
            contract_addr: anchorToken,
          },
        },
      ],
    },
  }, { logger: logger });
  astroportPair = result1.logs[0].eventsByType.from_contract.pair_contract_addr[0];
  astroportLiquidityToken = result1.logs[0].eventsByType.from_contract.liquidity_token_addr[0];
  console.log("success!");

  process.stdout.write("creating astroport LUNA-UST pair... ");
  const result2 = await executeContract(terra, deployer, astroportFactory, {
    create_pair: {
      pair_type: { xyk: {} },
      asset_infos: [
        {
          native_token: {
            denom: "uluna",
          },
        },
        {
          native_token: {
            denom: "uusd",
          },
        },
      ],
    },
  }, { logger: logger });
  const astroportPair2 = result2.logs[0].eventsByType.from_contract.pair_contract_addr[0];
  console.log("success!");

  process.stdout.write("alice provides initial liquidity to astroport pair... ");
  await executeContract(terra, alice, anchorToken, {
    increase_allowance: {
      amount: "69000000",
      spender: astroportPair,
    },
  }, { logger: logger });
  await executeContract(
    terra,
    alice,
    astroportPair,
    {
      provide_liquidity: {
        assets: [
          {
            info: {
              token: {
                contract_addr: anchorToken,
              },
            },
            amount: "69000000",
          },
          {
            info: {
              native_token: {
                denom: "uusd",
              },
            },
            amount: "420000000",
          },
        ],
      },
    },
    { coins: "420000000uusd", logger: logger }
  );
  console.log("success!");

  process.stdout.write("deploying mars oracle... ");
  oracle = await deployContract(terra, deployer, "../artifacts/mars_oracle.wasm", {
    owner: deployer.key.accAddress,
  });
  console.log("success!");

  process.stdout.write("configure spot price source with invalid pair, should fail... ");
  await expectPromiseToFail(
    executeContract(terra, deployer, oracle, {
      set_asset: {
        asset: {
          cw20: {
            contract_addr: anchorToken,
          },
        },
        price_source: {
          astroport_spot: {
            pair_address: astroportPair2, // we set price source for ANC but use the addr of LUNA-UST pair
          },
        },
      },
    }, { logger: logger })
  );
  console.log("success!");

  process.stdout.write("properly configure spot price source, should succeed... ");
  await executeContract(terra, deployer, oracle, {
    set_asset: {
      asset: {
        cw20: {
          contract_addr: anchorToken,
        },
      },
      price_source: {
        astroport_spot: {
          pair_address: astroportPair,
        },
      },
    },
  }, { logger: logger });
  console.log("success!");

  process.stdout.write("configure UST price source... ");
  await executeContract(terra, deployer, oracle, {
    set_asset: {
      asset: {
        native: {
          denom: "uusd",
        },
      },
      price_source: {
        fixed: {
          price: "1",
        },
      },
    },
  }, { logger: logger });
  console.log("success!");

  process.stdout.write("configure liquidity token price source... ");
  await executeContract(terra, deployer, oracle, {
    set_asset: {
      asset: {
        cw20: {
          contract_addr: astroportLiquidityToken,
        },
      },
      price_source: {
        astroport_liquidity_token: {
          pair_address: astroportPair,
        },
      },
    },
  }, { logger: logger });
  console.log("success!");

  // currently there are 69000000 uANC + 420000000 uusd in the pair. we calculating spot price by
  // attempting to swap PROBE_AMOUNT = 1000000 uANC to uusd
  // kValue = 69000000 * 420000000 = 28980000000000000
  // returnAmount = poolUusdDepth - kvalue / (poolUancDepth + offerUancAmount)
  // = 420000000 - 28980000000000000 / (69000000 + 1000000)
  // = 6000000
  // spotPrice = returnAmount / probeAmount = 6000000 / 1000000 = 6
  // we see the spot price is slightly less than the simple quotient (420 / 69 = 6.087) due to slippage
  process.stdout.write("querying spot price... ");
  await assertOraclePrice(anchorToken, "6");
  console.log("success!");

  // uanc price: 6 uusd
  // uanc depth = 69000000
  // uusd price: 1 uusd
  // uusd depth = 420000000
  // liquidity token supply = sqrt(69000000 * 420000000) = 170235131
  // liquidity token price = (6 * 69000000 + 1 * 420000000) / 170235131 = 4.89910628376700929(0)
  process.stdout.write("querying liquidity token price... ");
  await assertOraclePrice(astroportLiquidityToken, "4.89910628376700929");
  console.log("success!");

  // bob swap 1000000 uANC for uusd
  //
  // NOTE: the following calculations regarding tax assumes a tax rate of 0.1% and a cap of 1000000uusd.
  // this must be configured in LocalTerra/config/genesis.json
  //
  // fee = 6000000 * 0.003 = 18000
  // returnAmountAfterFee = 6000000 - 18000 = 5982000
  // bob receives uusd amount: deductTax(5982000) = 5976023
  // amount of uusd to deduct from pool balance: addTax(5976023) = 5981999
  // remaining pool balances:
  // uANC: 69000000 + 1000000 = 70000000
  // uusd: 420000000 - 5981999 = 414018001
  process.stdout.write("bob performs a swap to alter the price... ");
  await executeContract(terra, bob, anchorToken, {
    send: {
      contract: astroportPair,
      amount: "1000000",
      msg: toEncodedBinary({
        swap: {},
      }),
    },
  }, { logger: logger });
  const poolUusdDepth = await queryBalanceNative(terra, astroportPair, "uusd");
  strictEqual(poolUusdDepth, 414018001);
  const poolUancDepth = await queryBalanceCw20(terra, astroportPair, anchorToken);
  strictEqual(poolUancDepth, 70000000);
  console.log("success!");

  // kValue = 70000000 * 414018001 = 28981260070000000
  // returnAmount = poolUusdDepth - kvalue / (poolUancDepth + offerUancAmount)
  // = 414018001 - 28981260070000000 / (70000000 + 1000000)
  // = 5831240
  // spotPrice = returnAmount / probeAmount = 5831240 / 1000000 = 5.83124
  process.stdout.write("querying spot price... ");
  await assertOraclePrice(anchorToken, "5.83124");
  console.log("success!");

  process.stdout.write("configuring TWAP price source... ");
  await executeContract(terra, deployer, oracle, {
    set_asset: {
      asset: {
        cw20: {
          contract_addr: anchorToken,
        },
      },
      price_source: {
        astroport_twap: {
          pair_address: astroportPair,
          asset_address: anchorToken,
          window_size: 30,
          tolerance: 5, // will calculate average price over 30 +/- 5 seconds
        },
      },
    },
  }, { logger: logger });
  console.log("success!");

  let snapshots: Snapshot[] = [];

  process.stdout.write("recoding TWAP snapshot... ");
  snapshots.push(await recordTwapSnapshots());
  console.log("success!");

  // currently there is one snapshot, so querying price should fail
  process.stdout.write("expecting price query to fail... ");
  await expectPromiseToFail(assertOraclePrice(anchorToken, "0"));
  console.log("success!");

  // This line will probably fail, but it's not because of a smart contract bug, but rather of a
  // particularity in the way Terra LCD works.
  //
  // The oracle contracts forbids recoding two snapshots that are too close to each other.
  // Specifically, a new snapshot must be at least `tolerance` seconds apart from the latest
  // existing snapshot. This is to deter a potential DDoS attack to the contract's storage.
  //
  // The problem is how LocalTerra LCD estimates a transaction's gas cost. Right after a block is
  // included in the chain, there is a small delay in updating the context used to estimate gas.
  // That is, if we send a transaction right after after block n is mined, although we expect the tx
  // to be included in block n+1, the LCD will still use the context of block n to simulate the tx
  // in order to estimate gas.
  //
  // For this test, we have just recorded a snapshot in the previous block; let's say it's timestamp
  // is x. When estimating gas for the next snapshot, LCD still thinks the block timestamp is x,
  // rather than x + 5 (LocalTerra's block time is 5 seconds). Therefore, the check on the DDoS
  // fails, the transaction reverts, and LCD returns Error 400.
  //
  // The solution is simple: modify `createTransaction` function in helpers to explicitly feed in a
  // gas limit, so that LCD does not need to estimate it. The transaction should be successful.
  process.stdout.write("recoding TWAP snapshot... ");
  snapshots.push(await recordTwapSnapshots());
  console.log("success!");

  // currently there are two snapshots, but their timestamps are too close, so query should still fail
  process.stdout.write("expecting price query to fail... ");
  await expectPromiseToFail(assertOraclePrice(anchorToken, "0"));
  console.log("success!");

  // execute 3 swaps, and take a snapshot after each one
  for (let i = 0; i < 3; i++) {
    process.stdout.write("bob performs a swap to alter the price... ");
    await executeContract(terra, bob, anchorToken, {
      send: {
        contract: astroportPair,
        amount: "1000000",
        msg: toEncodedBinary({
          swap: {},
        }),
      },
    }, { logger: logger });
    console.log("success!");

    process.stdout.write("recoding TWAP snapshot... ");
    snapshots.push(await recordTwapSnapshots());
    console.log("success!");
  }

  // take a final snapshot
  process.stdout.write("recoding TWAP snapshot... ");
  snapshots.push(await recordTwapSnapshots());
  console.log("success!");

  // we have taken 6 snapshots. we query the average price immediately after the 6th snapshot was
  // taken, so the timestamp and cumulative price at the time of our query should be the same as the
  // 6th snapshot
  const snapshotEnd = snapshots[5];

  // Localterra uses ~5 seconds per block. therefore, the snapshots should have the following periods:
  // snapshots 1 & current: 40 seconds
  // snapshots 2 & current: 35 seconds (1 & 2 are in consecutive blocks, so 5 seconds apart)
  // snapshots 3 & current: 25 seconds (2 & 3 are 2 blocks apart so 10 seconds)
  // snapshots 4 & current: 15 seconds
  // snapshots 5 & current: 5 seconds
  // snapshots 6 & current: 0 seconds
  //
  // blocks 1, 2, 3 are within the tolerable window (30 +/- 10), within which 2 and 3 have the smallest
  // deviation from the desired window size. in this case the older snapshot is used
  //
  // in experience, the correct result should be 1 uanc = 5.6 uusd
  snapshots.sort((a, b) => {
    let diffA = diff(snapshotEnd.timestamp - a.timestamp, 30);
    let diffB = diff(snapshotEnd.timestamp - b.timestamp, 30);
    if (diffA < diffB) return -1;
    else if (diffA > diffB) return +1;
    return 0;
  });
  const snapshotStart = snapshots[0];

  const cumPriceDelta = snapshotEnd.cumulativePrice - snapshotStart.cumulativePrice;
  const period = snapshotEnd.timestamp - snapshotStart.timestamp;
  const expectedPrice = cumPriceDelta / period;

  process.stdout.write("querying TWAP average price... ");
  await assertOraclePrice(anchorToken, expectedPrice.toString());
  console.log("success!");

  console.log("OK");

  logger.showGasConsumption()
})();
