import { Coin, Int, isTxError, LocalTerra, MsgExecuteContract, StdFee } from "@terra-money/terra.js";
import {
  deployLiquidityPool,
  performTransaction,
  queryContract,
  setupLiquidityPool,
  toEncodedBinary
} from "./helpers.js";
import BigNumber from "bignumber.js";
import redis from "redis";
import { promisify } from "util";
import { strict as assert } from "assert";

BigNumber.config({DECIMAL_PLACES: 18})

// CONSTANTS AND GLOBALS
const INITIAL_ASSETS = [
  {denom: "uluna", borrow_slope: "4", loan_to_value: "0.5"},
  {denom: "uusd", borrow_slope: "5", loan_to_value: "0.8"},
  //{denom: "ukrw", borrow_slope: "2", loan_to_value: "0.6"},
];

function debug(string) {
  if (Number(process.env.DEBUG) === 1) {
    console.log(string);
  }
}

// ASSERTS
function assertEqualBN(left, right, message = "Expected values to be equal") {
  assert(left.eq(right), `${message} got \n\t-left:  ${left}, \n\t-right: ${right}`);
}

function assertEqualIndicesAndRates(expectedStateReserve, actualRates) {
  assertEqualBN(expectedStateReserve.borrowIndex, actualRates.borrowIndex);
  assertEqualBN(expectedStateReserve.liquidityIndex, actualRates.liquidityIndex);
  assertEqualBN(expectedStateReserve.borrowRate, actualRates.borrowRate);
  assertEqualBN(expectedStateReserve.liquidityRate, actualRates.liquidityRate);
}

// HELPERS
function isValueInDelta(value, target, deviation) {
  return Math.abs(value - target) < deviation
}

function getTimestampInSecondsFromDateField(dateField) {
  return (new Date(dateField).valueOf()) / 1000;
}

// Expected State
function updateExpectedAssetIndices(expectedState, asset, blockTime) {
  let expectedStateReserve = expectedState.reserves[asset];
  const SECONDS_PER_YEAR = new BigNumber(31536000);

  let secondsElapsed = blockTime - expectedStateReserve.interestsLastUpdated;

  let expectedAccumulatedLiquidityInterest =
    expectedStateReserve.liquidityRate
      .times(secondsElapsed)
      .dividedBy(SECONDS_PER_YEAR)
      .plus(1);
  expectedStateReserve.liquidityIndex = expectedStateReserve.liquidityIndex.times(expectedAccumulatedLiquidityInterest);

  let expectedAccumulatedBorrowInterest =
    expectedStateReserve.borrowRate
      .times(secondsElapsed)
      .dividedBy(SECONDS_PER_YEAR)
      .plus(1);
  expectedStateReserve.borrowIndex = expectedStateReserve.borrowIndex.times(expectedAccumulatedBorrowInterest);

  expectedStateReserve.interestsLastUpdated = blockTime;
}

function updateExpectedAssetRates(expectedState, asset) {
  let expectedStateReserve = expectedState.reserves[asset];

  let assetDebtTotal = expectedStateReserve.debtTotalScaled.times(expectedStateReserve.borrowIndex);
  let assetLiquidityTotal = new BigNumber(expectedState.lpContractBalances[asset]);
  let assetLockedTotal = assetLiquidityTotal.plus(assetDebtTotal);

  let expectedUtilizationRate =
    assetLockedTotal.isZero() ? new BigNumber(0) : assetDebtTotal.dividedBy(assetLockedTotal);

  expectedStateReserve.borrowRate = expectedUtilizationRate.times(expectedStateReserve.borrowSlope);
  expectedStateReserve.liquidityRate = expectedStateReserve.borrowRate.times(expectedUtilizationRate);
}

// QUERIES
async function getAddressNativeBalances(terra, address) {
  let ret = {};
  let balanceQuery =
    await terra.bank.balance(address);

  INITIAL_ASSETS.map(asset => asset.denom).forEach((denom) => {
    ret[denom] = Number(balanceQuery._coins[denom].amount);
  });

  return ret;
}


function getIndicesAndRatesFromTxResult(txResult) {
  let fromContractEvent = txResult.logs[0].eventsByType.from_contract;

  let liquidityRate = new BigNumber(fromContractEvent.liquidity_rate[0]);
  let borrowRate = new BigNumber(fromContractEvent.borrow_rate[0]);
  let liquidityIndex = new BigNumber(fromContractEvent.liquidity_index[0]);
  let borrowIndex = new BigNumber(fromContractEvent.borrow_index[0]);
  return {liquidityRate, borrowRate, liquidityIndex, borrowIndex}
}

// ACTIONS
async function depositAssets(terra, wallet, lpAddress, deposits) {
  for (let denom of Object.keys(deposits)) {
    let depositMsg = {"deposit_native": {"denom": denom}};
    let depositAmount = deposits[denom];
    let coins = new Coin(denom, depositAmount.toString());
    let executeDepositMsg = new MsgExecuteContract(wallet.key.accAddress, lpAddress, depositMsg, [coins]);

    await performTransaction(terra, wallet, executeDepositMsg);
  }
}

// TESTS
async function testDeposit(env, expectedState, depositUser, depositAsset, depositAmount) {
  console.log(`### Testing Deposit | ${depositUser} -> ${depositAmount} ${depositAsset}`);

  let depositAddress = env.terra.wallets[depositUser].key.accAddress;

  // Execute Deposit
  let depositMsg = {"deposit_native": {"denom": depositAsset}};
  let coins = new Coin(depositAsset, depositAmount);
  let executeDepositMsg =
    new MsgExecuteContract(depositAddress, env.lpAddress, depositMsg, [coins]);
  let depositTxResult = await performTransaction(env.terra, env.terra.wallets[depositUser], executeDepositMsg);
  debug(executeDepositMsg);
  debug(depositTxResult);

  let txInfo = await env.terra.tx.txInfo(depositTxResult.txhash);

  // lpContract balance should go up by deposit amount
  expectedState.lpContractBalances[depositAsset] += depositAmount;
  let lpContractBalance = await env.terra.bank.balance(env.lpAddress);
  debug(lpContractBalance);
  assert.strictEqual(expectedState.lpContractBalances[depositAsset], Number(lpContractBalance._coins[depositAsset].amount));

  // Update and check indices and rates
  let blockTime = getTimestampInSecondsFromDateField(txInfo.timestamp);
  updateExpectedAssetIndices(expectedState, depositAsset, blockTime);
  updateExpectedAssetRates(expectedState, depositAsset);

  let actualIndicesAndRates = getIndicesAndRatesFromTxResult(depositTxResult);
  assertEqualIndicesAndRates(expectedState.reserves[depositAsset], actualIndicesAndRates);

  // ma balance should go up by deposit amount
  expectedState.userBalances[depositUser].maBalances[depositAsset] += depositAmount / expectedState.reserves[depositAsset].liquidityIndex.toNumber();
  let balanceQueryMsg = {"balance": {"address": depositAddress}};
  const balanceQueryResult =
    await queryContract(
      env.terra,
      expectedState.reserves[depositAsset].maTokenAddress,
      balanceQueryMsg);
  debug(balanceQueryMsg);
  debug(balanceQueryResult);
  assert.strictEqual(expectedState.userBalances[depositUser].maBalances[depositAsset], Number(balanceQueryResult.balance));

  // Depositor balance should go down by deposit amount + txfee
  const depositTxFee = Number(txInfo.tx.fee.amount._coins[depositAsset].amount);
  expectedState.userBalances[depositUser].native_deposits[depositAsset] -= (depositAmount + depositTxFee);
  let actualEndingBalances = await getAddressNativeBalances(env.terra, depositAddress);
  assert.strictEqual(
    expectedState.userBalances[depositUser].native_deposits[depositAsset],
    actualEndingBalances[depositAsset]
  );
}

async function testRedeem(env, expectedState, redeemUser, redeemAsset, redeemAmount) {
  console.log(`### Testing Redeem | ${redeemUser} -> ${redeemAmount} ${redeemAsset}`);

  let redeemAddress = env.terra.wallets[redeemUser].key.accAddress;

  const executeMsg = {
    "send": {
      "contract": env.lpAddress,
      "amount": redeemAmount.toString(),
      "msg": toEncodedBinary({"redeem": {"id": redeemAsset}}),
    }
  };

  const redeemSendMsg = new MsgExecuteContract(redeemAddress, expectedState.reserves[redeemAsset].maTokenAddress, executeMsg);
  let redeemTxResult = await performTransaction(env.terra, env.terra.wallets[redeemUser], redeemSendMsg);
  debug(redeemSendMsg);
  debug(redeemTxResult);

  let redeemTxInfo = await env.terra.tx.txInfo(redeemTxResult.txhash);


  // Update and check indices and rates
  let blockTime = getTimestampInSecondsFromDateField(redeemTxInfo.timestamp);
  updateExpectedAssetIndices(expectedState, redeemAsset, blockTime);
  updateExpectedAssetRates(expectedState, redeemAsset);

  let actualIndicesAndRates = getIndicesAndRatesFromTxResult(redeemTxResult);
  assertEqualIndicesAndRates(expectedState.reserves[redeemAsset], actualIndicesAndRates);

  let expectedUnderlyingAssetAmount =
    expectedState.reserves[redeemAsset].liquidityIndex.mul(redeemAsset)

  // lpContract balance should go down by redeem amount adjusted by the liquidity index
  expectedState.lpContractBalances[redeemAsset] -= redeemAmount;
  let lpContractBalance = await env.terra.bank.balance(env.lpAddress);
  debug(lpContractBalance);
  assert.strictEqual(expectedState.lpContractBalances[redeemAsset], Number(lpContractBalance._coins[redeemAsset].amount));

  // user's ma balance should go down by redeem amount
  expectedState.userBalances[redeemUser].maBalances[redeemAsset] -= redeemAmount;
  let balanceQueryMsg = {"balance": {"address": redeemAddress}};
  const balanceQueryResult =
    await queryContract(
      env.terra,
      expectedState.reserves[redeemAsset].maTokenAddress,
      balanceQueryMsg);
  debug(balanceQueryMsg);
  debug(balanceQueryResult);
  assert.strictEqual(expectedState.userBalances[redeemUser].maBalances[redeemAsset], Number(balanceQueryResult.balance));

  // Redeemer balance should go up by redeem amount - txfee
  const redeemTxFee = Number(redeemTxInfo.tx.fee.amount._coins[redeemAsset].amount);
  expectedState.userBalances[redeemUser].native_deposits[redeemAsset] += (redeemAmount - redeemTxFee);
  let actualEndingBalances = await getAddressNativeBalances(env.terra, redeemAddress);
  assert.strictEqual(
    expectedState.userBalances[redeemUser].native_deposits[redeemAsset],
    actualEndingBalances[redeemAsset]
  );
}

async function testBorrow(env, expectedState, borrowUser, borrowAsset, borrowAmount) {
  console.log(`### Testing Borrow | ${borrowUser} -> ${borrowAmount} ${borrowAsset}`);

  let borrowAddress = env.terra.wallets[borrowUser].key.accAddress;

  let borrowMsg = {"borrow":
    {"asset": {"native": {"denom": borrowAsset}}, "amount": borrowAmount.toString()}
  };
  let executeBorrowMsg = new MsgExecuteContract(borrowAddress, env.lpAddress, borrowMsg);
  const borrowTxResult = await performTransaction(env.terra, env.terra.wallets[borrowUser], executeBorrowMsg);

  debug(executeBorrowMsg);
  debug(borrowTxResult);

  let borrowTxInfo = await env.terra.tx.txInfo(borrowTxResult.txhash);
  debug(borrowTxInfo);
  const borrowTxFee = Number(borrowTxInfo.tx.fee.amount._coins[borrowAsset].amount);

  // LP Contract balance should go down by borrow amount
  expectedState.lpContractBalances[borrowAsset] -= borrowAmount;
  const contractBalance = await env.terra.bank.balance(env.lpAddress);
  assert.strictEqual(expectedState.lpContractBalances[borrowAsset], Number(contractBalance._coins[borrowAsset].amount));

  // Update debt total, indices, and rates and test
  let borrowAmountScaled = new BigNumber(borrowAmount).dividedBy(expectedState.reserves[borrowAsset].borrowIndex);
  expectedState.reserves[borrowAsset].debtTotalScaled = expectedState.reserves[borrowAsset].debtTotalScaled.plus(borrowAmountScaled);
  let blockTime = getTimestampInSecondsFromDateField(borrowTxInfo.timestamp);
  updateExpectedAssetIndices(expectedState, borrowAsset, blockTime);
  updateExpectedAssetRates(expectedState, borrowAsset);

  let actualIndicesAndRates = getIndicesAndRatesFromTxResult(borrowTxResult);
  assertEqualIndicesAndRates(expectedState.reserves[borrowAsset], actualIndicesAndRates);

  // Borrower balance should go up by borrow amount - txfee
  expectedState.userBalances[borrowUser].native_deposits[borrowAsset] += (borrowAmount - borrowTxFee);
  let actualEndingBalances = await getAddressNativeBalances(env.terra, borrowAddress);
  assert.strictEqual(
    expectedState.userBalances[borrowUser].native_deposits[borrowAsset],
    actualEndingBalances[borrowAsset]
  );
}

async function testRepay(env, expectedState, repayUser, repayAsset, repayAmount) {
  console.log(`### Testing Repay | ${repayUser} -> ${repayAmount} ${repayAsset}`);

  let repayAddress = env.terra.wallets[repayUser].key.accAddress;

  const repayMsg = {"repay_native": {"denom": repayAsset}};
  let repayCoins = new Coin(repayAsset, repayAmount);
  const executeRepayMsg = new MsgExecuteContract(repayAddress, env.lpAddress, repayMsg, [repayCoins]);
  const repayTxResult = await performTransaction(env.terra, env.terra.wallets[repayUser], executeRepayMsg);

  debug(executeRepayMsg);
  debug(repayTxResult);

  let repayTxInfo = await env.terra.tx.txInfo(repayTxResult.txhash);

  // check lpContract balance increases by repay amount
  expectedState.lpContractBalances[repayAsset] += repayAmount;
  const contractBalance = await env.terra.bank.balance(env.lpAddress);
  assert.strictEqual(expectedState.lpContractBalances[repayAsset], Number(contractBalance._coins[repayAsset].amount));

  // Update debt total and check indices and rates
  let blockTime = getTimestampInSecondsFromDateField(repayTxInfo.timestamp);
  updateExpectedAssetIndices(expectedState, repayAsset, blockTime);
  let repayAmountScaled = new BigNumber(repayAmount).dividedBy(expectedState.reserves[repayAsset].borrowIndex);
  expectedState.reserves[repayAsset].debtTotalScaled = expectedState.reserves[repayAsset].debtTotalScaled.minus(repayAmountScaled);
  // expectedState.reserves[repayAsset].debtTotalScaled = new BigNumber(1);
  updateExpectedAssetRates(expectedState, repayAsset);

  let actualIndicesAndRates = getIndicesAndRatesFromTxResult(repayTxResult);
  assertEqualIndicesAndRates(expectedState.reserves[repayAsset], actualIndicesAndRates);

  // Repayer balance should go down by repay amount + txfee
  const repayTxFee = Number(repayTxInfo.tx.fee.amount._coins.uluna.amount);
  expectedState.userBalances[repayUser].native_deposits[repayAsset] -= (repayAmount + repayTxFee);
  let actualEndingBalances = await getAddressNativeBalances(env.terra, repayAddress);
  assert.strictEqual(
    expectedState.userBalances[repayUser].native_deposits[repayAsset],
    actualEndingBalances[repayAsset]
  );
}

async function testCollateralCheck(env, expectedState, user, deposits) {
  console.log(`### Testing CollateralCheck | ${user} -> ${deposits}`);
  let ltvDict = {};
  for (let asset of INITIAL_ASSETS) {
    if (deposits.hasOwnProperty(asset.denom)) {
      ltvDict[asset.denom] = asset.loan_to_value;
    }
  }

  await depositAssets(env.terra, env.terra.wallets[user], env.lpAddress, deposits);

  let {_coins: exchangeRates} = await env.terra.oracle.exchangeRates();
  let max_borrow_allowed_in_uluna = deposits.hasOwnProperty(("uluna")) ? deposits["uluna"] * ltvDict["uluna"] : 0;

  for (let denom of Object.keys(deposits)) {
    if (exchangeRates.hasOwnProperty(denom)) {
      max_borrow_allowed_in_uluna += ltvDict[denom] * deposits[denom] / exchangeRates[denom].amount;
    }
  }

  let max_borrow_allowed_in_uusd = new Int(max_borrow_allowed_in_uluna / exchangeRates['uusd'].amount);

  let excessiveBorrowAmount = max_borrow_allowed_in_uusd + 100;
  let validBorrowAmount = max_borrow_allowed_in_uusd - 100;

  let borrowMsg = {"borrow_native": {"denom": "uusd", "amount": excessiveBorrowAmount.toString()}};
  let executeBorrowMsg = new MsgExecuteContract(env.terra.wallets[user].key.accAddress, env.lpAddress, borrowMsg);
  let tx = await env.terra.wallets[user].createAndSignTx({
    msgs: [executeBorrowMsg],
    fee: new StdFee(30000000, [
      new Coin('uluna', 4000000),
      new Coin('uusd', 4000000),
      new Coin('ukrw', 4000000),
    ]),
  });

  const insufficientCollateralResult = await env.terra.tx.broadcast(tx);
  if (!isTxError(insufficientCollateralResult) || !insufficientCollateralResult.raw_log.includes("borrow amount exceeds maximum allowed given current collateral value")) {
    throw new Error("[Collateral]: Borrower has insufficient collateral and should not be able to borrow.");
  }

  borrowMsg = {"borrow_native": {"denom": "uusd", "amount": validBorrowAmount.toString()}};
  executeBorrowMsg = new MsgExecuteContract(env.terra.wallets[user].key.accAddress, env.lpAddress, borrowMsg);
  await performTransaction(env.terra, env.terra.wallets[user], executeBorrowMsg);

  debug(executeBorrowMsg);
}

function setRedisAsCacheSource(cache) {
  console.log("Will use Redis as cache");
  let redisClient = redis.createClient();
  redisClient.on("error", function(error) {
    console.log(`Redis client error: ${error}`);
  });

  const redisGet = promisify(redisClient.get).bind(redisClient);

  cache.active = true;
  cache.source = redisClient;
  cache.get = redisGet;
  cache.set = (k, v) => redisClient.set(k, v, redis.print);
}

// MAIN
async function main() {
  let cache = {active: false, source: null};

  if (process.env.CACHE === "redis") {
    setRedisAsCacheSource(cache);
  }

  if(!cache.active) {
    console.log("Not using cache, will deploy all contracts");
  }

  // Build cache
  if(cache.active) {
    cache.lpCodeId = Number(await cache.get("lpCodeId"));
    cache.cw20CodeId = Number(await cache.get("cw20CodeId"));
  }

  let terra = new LocalTerra();
  let ownerWallet = terra.wallets.test1;
  const lpDeployResults = await deployLiquidityPool(terra, ownerWallet, cache);

  // Store code ids in cache
  if(cache.active) {
    if(lpDeployResults.cw20CodeId !== cache.cw20CodeId) {
      cache.set("cw20CodeId", lpDeployResults.cw20CodeId);
    }
    if(lpDeployResults.lpCodeId !== cache.lpCodeId) {
      cache.set("lpCodeId", lpDeployResults.lpCodeId);
    }
  }

  let env = {
    terra,
    ownerWallet,
    lpAddress: lpDeployResults.lpAddress,
  };

  await setupLiquidityPool(env.terra, env.ownerWallet, env.lpAddress, {initialAssets: INITIAL_ASSETS});

  let test1NativeBalances = await getAddressNativeBalances(env.terra, env.terra.wallets.test1.key.accAddress);
  let test2NativeBalances = await getAddressNativeBalances(env.terra, env.terra.wallets.test2.key.accAddress);

  let expectedStateReserves = {};
  for (const denom of INITIAL_ASSETS.map(asset => asset.denom)) {
    let reserveQueryMsg = {"reserve": {"denom": denom}};
    let assetReserve = await queryContract(env.terra, env.lpAddress, reserveQueryMsg);

    expectedStateReserves[denom] = {
      liquidityRate: new BigNumber(0),
      borrowRate: new BigNumber(0),
      liquidityIndex: new BigNumber(1),
      borrowIndex: new BigNumber(1),
      debtTotalScaled: new BigNumber(0),
      borrowSlope: new BigNumber(assetReserve.borrow_slope),
      interestsLastUpdated: assetReserve.interests_last_updated,
      maTokenAddress: assetReserve.ma_token_address,
    };
  }

  let expectedState = {
    lpContractBalances: {
      uluna: 0,
      uusd: 0,
      //ukrw: 0,
    },
    userBalances: {
      test1: {
        native_deposits: test1NativeBalances,
        maBalances: {
          uluna: 0,
          uusd: 0,
          //ukrw: 0,
        }
      },
      test2: {
        native_deposits: test2NativeBalances,
        maBalances: {
          uluna: 0,
          uusd: 0,
          //ukrw: 0,
        }},
    },
    reserves: expectedStateReserves,
  }

  let deposits = {uluna: 10_000_000, uusd: 5_000_000};//, ukrw: 50_000_000};

  await testDeposit(env, expectedState, "test1", "uluna", 10_000_000);
  await testDeposit(env, expectedState, "test2", "uusd", 10_000_000);
  await testBorrow(env, expectedState, "test1", "uusd", 2_000_000);
  await testRedeem(env, expectedState, "test1", "uluna", 3_000_000);
  await testRepay(env, expectedState, "test1", "uusd", 1_000_000);
  await testCollateralCheck(env, expectedState, "test2", deposits);
  console.log("OK");
}

main().catch(err => console.log(err));
