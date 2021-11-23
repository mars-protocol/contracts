import { Coin, isTxError, LocalTerra, MsgExecuteContract, StdFee } from "@terra-money/terra.js";
import { deployBasecampContract, performTransaction, queryContract, toEncodedBinary } from "./helpers.js";
import { strict as assert } from 'assert';

// check token symbols
async function testTokenInit(env) {
  console.log("### Testing Token Info...");
  let {mars_token_address, xmars_token_address} = env.cw20_contracts;

  let queryTokenInfoMsg = {"token_info": {}};
  let {symbol: marsSymbol} = await queryContract(env.terra, mars_token_address, queryTokenInfoMsg);
  assert.deepEqual(marsSymbol, "Mars");

  let {symbol: xMarsSymbol} = await queryContract(env.terra, xmars_token_address, queryTokenInfoMsg);
  assert.deepEqual(xMarsSymbol, "xMars");

  // check minter for both contracts is the basecamp contract
  console.log("### Testing Minter...");
  let queryMinterMsg = {"minter": {}};
  let {minter: marsMinter} = await queryContract(env.terra, mars_token_address, queryMinterMsg);
  assert.deepEqual(marsMinter, env.basecampContractAddress);

  let {minter: xMarsMinter} = await queryContract(env.terra, xmars_token_address, queryMinterMsg);
  assert.deepEqual(xMarsMinter, env.basecampContractAddress);
}

// mint tokens to recipient
async function testMint(env, expectedState, recipient, mintAmount) {
  let {mars_token_address} = env.cw20_contracts;

  let mintMsg = {"mint_mars": {"recipient": env.terra.wallets[recipient].key.accAddress, "amount": mintAmount.toString()}};
  let mintSendMsg = new MsgExecuteContract(env.terra.wallets[recipient].key.accAddress, env.basecampContractAddress, mintMsg);
  await performTransaction(env.terra, env.terra.wallets[recipient], mintSendMsg);

  expectedState.userBalance[recipient].marsBalance += mintAmount;
  let balanceQueryMsg = {"balance": {"address": env.terra.wallets[recipient].key.accAddress}};
  const {balance: balanceAfterMint} = await queryContract(env.terra, mars_token_address, balanceQueryMsg);
  assert.deepEqual(Number(balanceAfterMint), expectedState.userBalance[recipient].marsBalance);
}

// stake tokens -> send mars and receive xmars
async function testStake(env, expectedState, staker, stakeAmount) {
  console.log("### Testing Stake...");
  let {mars_token_address, xmars_token_address} = env.cw20_contracts;
  const stakeExecuteMsg = {
    "send": {
      "contract": env.basecampContractAddress,
      "amount": stakeAmount.toString(),
      "msg": toEncodedBinary("stake"),
    }
  };
  let stakeSendMsg = new MsgExecuteContract(env.terra.wallets[staker].key.accAddress, mars_token_address, stakeExecuteMsg);
  await performTransaction(env.terra, env.terra.wallets[staker], stakeSendMsg);

  let basecampBalanceQueryMsg = {"balance": {"address": env.basecampContractAddress}};
  let {balance: totalMarsInBasecamp} = await queryContract(env.terra, mars_token_address, basecampBalanceQueryMsg);

  let queryTokenInfoMsg = {"token_info": {}};
  let {total_supply: totalXMarsSupply} = await queryContract(env.terra, xmars_token_address, queryTokenInfoMsg);


  let expectedXMarsReturned;

  if (Number(totalMarsInBasecamp) === 0 || Number(totalXMarsSupply) === 0) {
    expectedXMarsReturned = stakeAmount;
  } else {
    expectedXMarsReturned = stakeAmount * totalXMarsSupply / totalMarsInBasecamp;
  }

  expectedState.userBalance[staker].marsBalance -= stakeAmount;
  expectedState.userBalance[staker].xMarsBalance += expectedXMarsReturned;

  expectedState.basecampBalance.marsBalance += stakeAmount;

  // provide mars and receive xmars tokens
  let balanceQueryMsg = {"balance": {"address": env.terra.wallets[staker].key.accAddress}};
  const {balance: stakerBalanceAfterStake} = await queryContract(env.terra, mars_token_address, balanceQueryMsg);
  assert.deepEqual(Number(stakerBalanceAfterStake), expectedState.userBalance[staker].marsBalance);

  const {balance: xMarsBalanceAfterStake} = await queryContract(env.terra, xmars_token_address, balanceQueryMsg);
  assert.deepEqual(Number(xMarsBalanceAfterStake), expectedState.userBalance[staker].xMarsBalance);

  // basecamp receives mars
  const {balance: basecampBalanceAfterStake} = await queryContract(env.terra, mars_token_address, basecampBalanceQueryMsg);
  assert.deepEqual(Number(basecampBalanceAfterStake), expectedState.basecampBalance.marsBalance);

  // trying to unstake without activating cooldown should fail
  const unstakeExecuteMsg = {
    "send": {
      "contract": env.basecampContractAddress,
      "amount": (0.5 * stakeAmount).toString(),
      "msg": toEncodedBinary("unstake"),
    }
  };

  let unstakeSendMsg = new MsgExecuteContract(env.terra.wallets[staker].key.accAddress, xmars_token_address, unstakeExecuteMsg);
  const tx = await env.terra.wallets[staker].createAndSignTx({
    msgs: [unstakeSendMsg],
    fee: new StdFee(30000000, [
      new Coin('uluna', 4500000),
    ]),
  });
  const result = await env.terra.tx.broadcast(tx);
  assert(isTxError(result));
}


// activate cooldown
async function activateCooldown(env, user) {
  console.log("### Testing Cooldown...");
  let cooldownMsg = {"cooldown": {}};
  let cooldownSendMsg = new MsgExecuteContract(env.terra.wallets[user].key.accAddress, env.basecampContractAddress, cooldownMsg);
  await performTransaction(env.terra, env.terra.wallets[user], cooldownSendMsg);
}


// unstake tokens -> send xmars to be burnt and receive mars in return
async function testUnstake(env, expectedState, unstaker, unstakeAmount) {
  console.log("### Testing Unstake...");
  let {mars_token_address, xmars_token_address} = env.cw20_contracts;
  const unstakeExecuteMsg = {
    "send": {
      "contract": env.basecampContractAddress,
      "amount": unstakeAmount.toString(),
      "msg": toEncodedBinary("unstake"),
    }
  };

  let unstakeSendMsg = new MsgExecuteContract(env.terra.wallets[unstaker].key.accAddress, xmars_token_address, unstakeExecuteMsg);
  await performTransaction(env.terra, env.terra.wallets[unstaker], unstakeSendMsg);

  //  unstake_amount = burn_amount * total_mars_in_basecamp / total_xmars_supply
  let basecampBalanceQueryMsg = {"balance": {"address": env.basecampContractAddress}};
  let {balance: totalMarsInBasecamp} = await queryContract(env.terra, mars_token_address, basecampBalanceQueryMsg);

  let queryTokenInfoMsg = {"token_info": {}};
  let {total_supply: totalXMarsSupply} = await queryContract(env.terra, xmars_token_address, queryTokenInfoMsg);

  let expectedUnstakeAmount = unstakeAmount * Number(totalMarsInBasecamp) / Number(totalXMarsSupply);

  expectedState.userBalance[unstaker].marsBalance += expectedUnstakeAmount;
  expectedState.basecampBalance.marsBalance -= expectedUnstakeAmount;

  expectedState.userBalance[unstaker].xMarsBalance -= unstakeAmount;

  let balanceQueryMsg = {"balance": {"address": env.terra.wallets[unstaker].key.accAddress}};
  const {balance: marsBalanceAfterUnstake} = await queryContract(env.terra, mars_token_address, balanceQueryMsg);
  const {balance: basecampBalanceAfterUnstake} = await queryContract(env.terra, mars_token_address, basecampBalanceQueryMsg);

  assert.deepEqual(Number(marsBalanceAfterUnstake), expectedState.userBalance[unstaker].marsBalance);
  assert.deepEqual(Number(basecampBalanceAfterUnstake), expectedState.basecampBalance.marsBalance);

  const {balance: xMarsBalanceAfterUnstake} = await queryContract(env.terra, xmars_token_address, balanceQueryMsg);
  assert.deepEqual(Number(xMarsBalanceAfterUnstake), expectedState.userBalance[unstaker].xMarsBalance);
}

async function main() {
  let terra = new LocalTerra();
  let wallet = terra.wallets.test1;

  let cooldownDuration = 1;
  let unstakeWindow = 30;
  let basecampContractAddress = await deployBasecampContract(terra, wallet, cooldownDuration, unstakeWindow);
  // query config for mars and xmars contracts
  let queryConfigMsg = {"config": {}};
  let cw20_contracts = await terra.wasm.contractQuery(basecampContractAddress, queryConfigMsg);

  let env = {
    terra,
    wallet,
    basecampContractAddress,
    cw20_contracts,
  };

  let expectedState = {
    userBalance: {
      test1: {
        marsBalance: 0,
        xMarsBalance: 0,
      }
    },
    basecampBalance: {
      marsBalance: 0,
    },
  };

  await testTokenInit(env);
  await testMint(env,  expectedState, "test1", 10_000_000);
  await testStake(env, expectedState, "test1", 1_000_000);
  await activateCooldown(env, "test1");
  await testUnstake(env, expectedState, "test1", 100_000);

  console.log("OK");
}

main().catch(console.log)
