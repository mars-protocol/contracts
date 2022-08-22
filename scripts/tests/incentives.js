var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { LocalTerra, MnemonicKey } from '@terra-money/terra.js';
import { strictEqual } from 'assert';
import { join } from 'path';
import 'dotenv/config.js';
import { deployContract, executeContract, Logger, setTimeoutDuration, uploadContract } from '../helpers.js';
import { depositNative, getTxTimestamp, queryBalanceCw20, queryMaAssetAddress, setAssetOraclePriceSource, transferCw20, withdraw, } from './test_helpers.js';
// CONSTS
// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH;
const INCENTIVES_UMARS_BALANCE = 1000000000000;
const ULUNA_UMARS_EMISSION_RATE = 2000000;
const UUSD_UMARS_EMISSION_RATE = 4000000;
const MA_TOKEN_SCALING_FACTOR = 1000000;
// multiples of coins to deposit and withdraw from the red bank
const X = 10000000000;
// HELPERS
function setAssetIncentive(terra, wallet, incentives, maTokenAddress, umarsEmissionRate, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, incentives, {
            set_asset_incentive: {
                ma_token_address: maTokenAddress,
                emission_per_second: String(umarsEmissionRate),
            },
        }, { logger: logger });
    });
}
function claimRewards(terra, wallet, incentives, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const result = yield executeContract(terra, wallet, incentives, { claim_rewards: {} }, { logger: logger });
        return yield getTxTimestamp(terra, result);
    });
}
function computeExpectedRewards(startTime, endTime, umarsRate) {
    return (endTime - startTime) * umarsRate;
}
function assertBalance(balance, expectedBalance) {
    return strictEqual(balance, Math.floor(expectedBalance));
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    // SETUP
    setTimeoutDuration(100);
    const logger = new Logger();
    const terra = new LocalTerra();
    // addresses
    const deployer = terra.wallets.test1;
    const alice = terra.wallets.test2;
    const bob = terra.wallets.test3;
    const carol = terra.wallets.test4;
    const dan = terra.wallets.test5;
    // mock contract addresses
    const protocolRewardsCollector = new MnemonicKey().accAddress;
    const astroportFactory = new MnemonicKey().accAddress;
    console.log('upload contracts');
    const addressProvider = yield deployContract(terra, deployer, '../artifacts/mars_address_provider.wasm', {
        owner: deployer.key.accAddress,
    });
    const incentives = yield deployContract(terra, deployer, '../artifacts/mars_incentives.wasm', {
        owner: deployer.key.accAddress,
        address_provider_address: addressProvider,
    });
    const oracle = yield deployContract(terra, deployer, '../artifacts/mars_oracle.wasm', {
        owner: deployer.key.accAddress,
    });
    const maTokenCodeId = yield uploadContract(terra, deployer, '../artifacts/mars_ma_token.wasm');
    const redBank = yield deployContract(terra, deployer, '../artifacts/mars_red_bank.wasm', {
        config: {
            owner: deployer.key.accAddress,
            address_provider_address: addressProvider,
            safety_fund_fee_share: '0.1',
            treasury_fee_share: '0.2',
            ma_token_code_id: maTokenCodeId,
            close_factor: '0.5',
        },
    });
    const staking = yield deployContract(terra, deployer, '../artifacts/mars_staking.wasm', {
        config: {
            owner: deployer.key.accAddress,
            address_provider_address: addressProvider,
            astroport_factory_address: astroportFactory,
            astroport_max_spread: '0.05',
            cooldown_duration: 10,
            unstake_window: 300,
        },
    });
    const mars = yield deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, 'cw20_base.wasm'), {
        name: 'Mars',
        symbol: 'MARS',
        decimals: 6,
        initial_balances: [{ address: incentives, amount: String(INCENTIVES_UMARS_BALANCE) }],
        mint: { minter: incentives },
    });
    const xMars = yield deployContract(terra, deployer, '../artifacts/mars_xmars_token.wasm', {
        name: 'xMars',
        symbol: 'xMARS',
        decimals: 6,
        initial_balances: [],
        mint: { minter: staking },
    });
    // update address provider
    yield executeContract(terra, deployer, addressProvider, {
        update_config: {
            config: {
                owner: deployer.key.accAddress,
                incentives_address: incentives,
                mars_token_address: mars,
                oracle_address: oracle,
                protocol_rewards_collector_address: protocolRewardsCollector,
                red_bank_address: redBank,
                staking_address: staking,
                xmars_token_address: xMars,
                protocol_admin_address: deployer.key.accAddress,
            },
        },
    }, { logger: logger });
    console.log('init assets');
    // uluna
    yield executeContract(terra, deployer, redBank, {
        init_asset: {
            asset: { native: { denom: 'uluna' } },
            asset_params: {
                initial_borrow_rate: '0.1',
                max_loan_to_value: '0.55',
                reserve_factor: '0.2',
                liquidation_threshold: '0.65',
                liquidation_bonus: '0.1',
                interest_rate_model_params: {
                    dynamic: {
                        min_borrow_rate: '0.0',
                        max_borrow_rate: '2.0',
                        kp_1: '0.02',
                        optimal_utilization_rate: '0.7',
                        kp_augmentation_threshold: '0.15',
                        kp_2: '0.05',
                        update_threshold_txs: 1,
                        update_threshold_seconds: 600,
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    yield setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: 'uluna' } }, 25, logger);
    const maUluna = yield queryMaAssetAddress(terra, redBank, { native: { denom: 'uluna' } });
    // uusd
    yield executeContract(terra, deployer, redBank, {
        init_asset: {
            asset: { native: { denom: 'uusd' } },
            asset_params: {
                initial_borrow_rate: '0.2',
                max_loan_to_value: '0.75',
                reserve_factor: '0.2',
                liquidation_threshold: '0.85',
                liquidation_bonus: '0.1',
                interest_rate_model_params: {
                    dynamic: {
                        min_borrow_rate: '0.0',
                        max_borrow_rate: '1.0',
                        kp_1: '0.04',
                        optimal_utilization_rate: '0.9',
                        kp_augmentation_threshold: '0.15',
                        kp_2: '0.07',
                        update_threshold_txs: 1,
                        update_threshold_seconds: 600,
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    yield setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: 'uusd' } }, 1, logger);
    const maUusd = yield queryMaAssetAddress(terra, redBank, { native: { denom: 'uusd' } });
    // TESTS
    console.log('alice deposits uusd before any incentive is set for uusd');
    yield depositNative(terra, alice, redBank, 'uusd', X, logger);
    console.log('set incentives');
    yield setAssetIncentive(terra, deployer, incentives, maUluna, ULUNA_UMARS_EMISSION_RATE, logger);
    let result = yield setAssetIncentive(terra, deployer, incentives, maUusd, UUSD_UMARS_EMISSION_RATE, logger);
    const uusdIncentiveStartTime = yield getTxTimestamp(terra, result);
    console.log('users deposit assets');
    result = yield depositNative(terra, alice, redBank, 'uluna', X, logger);
    const aliceLunaDepositTime = yield getTxTimestamp(terra, result);
    result = yield depositNative(terra, bob, redBank, 'uluna', X, logger);
    const bobLunaDepositTime = yield getTxTimestamp(terra, result);
    result = yield depositNative(terra, carol, redBank, 'uluna', 2 * X, logger);
    const carolLunaDepositTime = yield getTxTimestamp(terra, result);
    result = yield depositNative(terra, dan, redBank, 'uusd', X, logger);
    const danUsdDepositTime = yield getTxTimestamp(terra, result);
    const aliceClaimRewardsTime = yield claimRewards(terra, alice, incentives, logger);
    let aliceXmarsBalance = yield queryBalanceCw20(terra, alice.key.accAddress, xMars);
    let expectedAliceXmarsBalance = computeExpectedRewards(aliceLunaDepositTime, bobLunaDepositTime, ULUNA_UMARS_EMISSION_RATE) +
        computeExpectedRewards(bobLunaDepositTime, carolLunaDepositTime, ULUNA_UMARS_EMISSION_RATE / 2) +
        computeExpectedRewards(carolLunaDepositTime, aliceClaimRewardsTime, ULUNA_UMARS_EMISSION_RATE / 4) +
        computeExpectedRewards(uusdIncentiveStartTime, danUsdDepositTime, UUSD_UMARS_EMISSION_RATE) +
        computeExpectedRewards(danUsdDepositTime, aliceClaimRewardsTime, UUSD_UMARS_EMISSION_RATE / 2);
    assertBalance(aliceXmarsBalance, expectedAliceXmarsBalance);
    const bobClaimRewardsTime = yield claimRewards(terra, bob, incentives, logger);
    let bobXmarsBalance = yield queryBalanceCw20(terra, bob.key.accAddress, xMars);
    let expectedBobXmarsBalance = computeExpectedRewards(bobLunaDepositTime, carolLunaDepositTime, ULUNA_UMARS_EMISSION_RATE / 2) +
        computeExpectedRewards(carolLunaDepositTime, bobClaimRewardsTime, ULUNA_UMARS_EMISSION_RATE / 4);
    assertBalance(bobXmarsBalance, expectedBobXmarsBalance);
    const carolClaimRewardsTime = yield claimRewards(terra, carol, incentives, logger);
    const carolXmarsBalance = yield queryBalanceCw20(terra, carol.key.accAddress, xMars);
    const expectedCarolXmarsBalance = computeExpectedRewards(carolLunaDepositTime, carolClaimRewardsTime, ULUNA_UMARS_EMISSION_RATE / 2);
    assertBalance(carolXmarsBalance, expectedCarolXmarsBalance);
    const danClaimRewardsTime = yield claimRewards(terra, dan, incentives, logger);
    const danXmarsBalance = yield queryBalanceCw20(terra, dan.key.accAddress, xMars);
    const expectedDanXmarsBalance = computeExpectedRewards(danUsdDepositTime, danClaimRewardsTime, UUSD_UMARS_EMISSION_RATE / 2);
    assertBalance(danXmarsBalance, expectedDanXmarsBalance);
    console.log('turn off uluna incentives');
    result = yield executeContract(terra, deployer, incentives, {
        set_asset_incentive: {
            ma_token_address: maUluna,
            emission_per_second: '0',
        },
    }, { logger: logger });
    const ulunaIncentiveEndTime = yield getTxTimestamp(terra, result);
    // Bob accrues rewards for uluna until the rewards were turned off
    yield claimRewards(terra, bob, incentives, logger);
    bobXmarsBalance = yield queryBalanceCw20(terra, bob.key.accAddress, xMars);
    expectedBobXmarsBalance += computeExpectedRewards(bobClaimRewardsTime, ulunaIncentiveEndTime, ULUNA_UMARS_EMISSION_RATE / 4);
    assertBalance(bobXmarsBalance, expectedBobXmarsBalance);
    // Alice accrues rewards for uluna until the rewards were turned off,
    // and continues to accrue rewards for uusd
    const aliceClaimRewardsTime2 = yield claimRewards(terra, alice, incentives, logger);
    aliceXmarsBalance = yield queryBalanceCw20(terra, alice.key.accAddress, xMars);
    expectedAliceXmarsBalance +=
        computeExpectedRewards(aliceClaimRewardsTime, ulunaIncentiveEndTime, ULUNA_UMARS_EMISSION_RATE / 4) +
            computeExpectedRewards(aliceClaimRewardsTime, aliceClaimRewardsTime2, UUSD_UMARS_EMISSION_RATE / 2);
    assertBalance(aliceXmarsBalance, expectedAliceXmarsBalance);
    console.log('transfer maUusd');
    result = yield transferCw20(terra, alice, maUusd, bob.key.accAddress, (X / 2) * MA_TOKEN_SCALING_FACTOR, logger);
    const uusdTransferTime = yield getTxTimestamp(terra, result);
    // Alice accrues rewards for X uusd until transferring X/2 uusd to Bob,
    // then accrues rewards for X/2 uusd
    const aliceClaimRewardsTime3 = yield claimRewards(terra, alice, incentives, logger);
    aliceXmarsBalance = yield queryBalanceCw20(terra, alice.key.accAddress, xMars);
    expectedAliceXmarsBalance +=
        computeExpectedRewards(aliceClaimRewardsTime2, uusdTransferTime, UUSD_UMARS_EMISSION_RATE / 2) +
            computeExpectedRewards(uusdTransferTime, aliceClaimRewardsTime3, UUSD_UMARS_EMISSION_RATE / 4);
    assertBalance(aliceXmarsBalance, expectedAliceXmarsBalance);
    // Bob accrues rewards for uusd after receiving X/2 uusd from Alice
    const bobClaimRewardsTime3 = yield claimRewards(terra, bob, incentives, logger);
    bobXmarsBalance = yield queryBalanceCw20(terra, bob.key.accAddress, xMars);
    expectedBobXmarsBalance += computeExpectedRewards(uusdTransferTime, bobClaimRewardsTime3, UUSD_UMARS_EMISSION_RATE / 4);
    assertBalance(bobXmarsBalance, expectedBobXmarsBalance);
    console.log('withdraw uusd');
    result = yield withdraw(terra, alice, redBank, { native: { denom: 'uusd' } }, X / 2, logger);
    const aliceWithdrawUusdTime = yield getTxTimestamp(terra, result);
    result = yield withdraw(terra, bob, redBank, { native: { denom: 'uusd' } }, X / 2, logger);
    const bobWithdrawUusdTime = yield getTxTimestamp(terra, result);
    // Alice accrues rewards for X/2 uusd until withdrawing
    yield claimRewards(terra, alice, incentives, logger);
    aliceXmarsBalance = yield queryBalanceCw20(terra, alice.key.accAddress, xMars);
    expectedAliceXmarsBalance += computeExpectedRewards(aliceClaimRewardsTime3, aliceWithdrawUusdTime, UUSD_UMARS_EMISSION_RATE / 4);
    assertBalance(aliceXmarsBalance, expectedAliceXmarsBalance);
    // Bob accrues rewards for X/2 uusd until withdrawing
    yield claimRewards(terra, bob, incentives, logger);
    bobXmarsBalance = yield queryBalanceCw20(terra, bob.key.accAddress, xMars);
    expectedBobXmarsBalance +=
        computeExpectedRewards(bobClaimRewardsTime3, aliceWithdrawUusdTime, UUSD_UMARS_EMISSION_RATE / 4) +
            computeExpectedRewards(aliceWithdrawUusdTime, bobWithdrawUusdTime, UUSD_UMARS_EMISSION_RATE / 3);
    assertBalance(bobXmarsBalance, expectedBobXmarsBalance);
    console.log('OK');
    logger.showGasConsumption();
}))();
