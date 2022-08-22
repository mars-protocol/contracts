var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
/*
Tests that market utilization rates update when funds are sent to/from the red bank
*/
import { LocalTerra, MnemonicKey } from '@terra-money/terra.js';
import { join } from 'path';
import { strictEqual, strict as assert } from 'assert';
import 'dotenv/config.js';
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, toEncodedBinary, uploadContract, } from '../helpers.js';
import { approximateEqual, queryMaAssetAddress } from './test_helpers.js';
// CONSTS
// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH;
const UUSD_LINEAR_INTEREST_RATE_SLOPE_1 = 2.5;
const UUSD_MAX_LTV = 0.75;
const MARS_OPTIMAL_UTILIZATION_RATE = 0.5;
const MARS_MAX_LTV = 0.55;
const MARS_USD_PRICE = 2;
const ALICE_UUSD_COLLATERAL = 10000000000;
const ALICE_MARS_COLLATERAL = 10000000000;
const BOB_UUSD_COLLATERAL = 100000000000;
const BOB_MARS_COLLATERAL = 100000000000;
// HELPERS
function queryBorrowRate(terra, redBank, asset) {
    return __awaiter(this, void 0, void 0, function* () {
        const market = yield queryContract(terra, redBank, { market: { asset } });
        return parseFloat(market.borrow_rate);
    });
}
// TESTS
function testLinearInterestRate(env, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const { terra, redBank, mars, maUusd, maMars, alice, bob } = env;
        console.log('alice deposits uusd');
        yield executeContract(terra, alice, redBank, { deposit_native: { denom: 'uusd' } }, { coins: `${ALICE_UUSD_COLLATERAL}uusd`, logger: logger });
        console.log('bob deposits mars');
        yield executeContract(terra, bob, mars, {
            send: {
                contract: redBank,
                amount: String(BOB_MARS_COLLATERAL),
                msg: toEncodedBinary({ deposit_cw20: {} }),
            },
        }, { logger: logger });
        console.log('bob borrows uusd');
        yield executeContract(terra, bob, redBank, {
            borrow: {
                asset: { native: { denom: 'uusd' } },
                // TODO change this to borrow `ALICE_UUSD_COLLATERAL` once borrowing exact liquidity amount
                // bug is fixed
                amount: String(ALICE_UUSD_COLLATERAL - 1),
            },
        }, { logger: logger });
        let uusdBorrowRate = yield queryBorrowRate(terra, redBank, { native: { denom: 'uusd' } });
        // rate will be approximately the slope rate because almost all liquidity has been borrowed
        approximateEqual(uusdBorrowRate, UUSD_LINEAR_INTEREST_RATE_SLOPE_1, 0.01);
        console.log('alice deposits uusd');
        yield executeContract(terra, alice, redBank, { deposit_native: { denom: 'uusd' } }, { coins: `${3 * ALICE_UUSD_COLLATERAL}uusd`, logger: logger });
        uusdBorrowRate = yield queryBorrowRate(terra, redBank, { native: { denom: 'uusd' } });
        // rate will be approximately a quarter of the slope rate because a quarter of the liquidity has
        // been borrowed
        approximateEqual(uusdBorrowRate, UUSD_LINEAR_INTEREST_RATE_SLOPE_1 / 4, 0.01);
        console.log('alice withdraws uusd');
        yield executeContract(terra, alice, redBank, {
            withdraw: {
                asset: { native: { denom: 'uusd' } },
                amount: String(3 * ALICE_UUSD_COLLATERAL),
            },
        }, { logger: logger });
        uusdBorrowRate = yield queryBorrowRate(terra, redBank, { native: { denom: 'uusd' } });
        // rate will be approximately the slope rate because almost all liquidity has been borrowed
        approximateEqual(uusdBorrowRate, UUSD_LINEAR_INTEREST_RATE_SLOPE_1, 0.01);
        console.log('bob repays uusd');
        yield executeContract(terra, bob, redBank, { repay_native: { denom: 'uusd' } }, { coins: `${0.8 * ALICE_UUSD_COLLATERAL}uusd`, logger: logger });
        uusdBorrowRate = yield queryBorrowRate(terra, redBank, { native: { denom: 'uusd' } });
        // rate will be approximately a fifth of the slope rate because a fifth of the liquidity has
        // been borrowed
        approximateEqual(uusdBorrowRate, UUSD_LINEAR_INTEREST_RATE_SLOPE_1 / 5, 0.01);
        // withdraw all liquidity to reset the red-bank before the next test
        yield executeContract(terra, bob, redBank, { repay_native: { denom: 'uusd' } }, { coins: `${10 * ALICE_UUSD_COLLATERAL}uusd`, logger: logger });
        yield executeContract(terra, bob, redBank, { withdraw: { asset: { cw20: { contract_addr: mars } } } }, { logger: logger });
        yield executeContract(terra, alice, redBank, { withdraw: { asset: { native: { denom: 'uusd' } } } }, { logger: logger });
        const maUusdTokenInfo = yield queryContract(terra, maUusd, { token_info: {} });
        strictEqual(parseInt(maUusdTokenInfo.total_supply), 0);
        const maMarsTokenInfo = yield queryContract(terra, maMars, { token_info: {} });
        strictEqual(parseInt(maMarsTokenInfo.total_supply), 0);
    });
}
function testDynamicInterestRate(env, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const { terra, redBank, mars, alice, bob } = env;
        console.log('alice deposits mars');
        yield executeContract(terra, alice, mars, {
            send: {
                contract: redBank,
                amount: String(ALICE_MARS_COLLATERAL),
                msg: toEncodedBinary({ deposit_cw20: {} }),
            },
        }, { logger: logger });
        console.log('bob deposits uusd');
        yield executeContract(terra, bob, redBank, { deposit_native: { denom: 'uusd' } }, { coins: `${BOB_UUSD_COLLATERAL}uusd`, logger: logger });
        console.log('bob borrows mars');
        yield executeContract(terra, bob, redBank, {
            borrow: {
                asset: { cw20: { contract_addr: mars } },
                amount: String(ALICE_MARS_COLLATERAL),
            },
        }, { logger: logger });
        let marsBorrowRateBefore = yield queryBorrowRate(terra, redBank, { cw20: { contract_addr: mars } });
        console.log('alice deposits mars');
        yield executeContract(terra, alice, mars, {
            send: {
                contract: redBank,
                amount: String(3 * ALICE_MARS_COLLATERAL),
                msg: toEncodedBinary({ deposit_cw20: {} }),
            },
        }, { logger: logger });
        let marsBorrowRateAfter = yield queryBorrowRate(terra, redBank, { cw20: { contract_addr: mars } });
        // the new rate should be lower than the previous rate because the new utilization rate is lower
        // than the optimal utilization rate
        assert(marsBorrowRateAfter < marsBorrowRateBefore);
        console.log('alice withdraws mars');
        marsBorrowRateBefore = marsBorrowRateAfter;
        yield executeContract(terra, alice, redBank, {
            withdraw: {
                asset: { cw20: { contract_addr: mars } },
                amount: String(3 * ALICE_MARS_COLLATERAL),
            },
        }, { logger: logger });
        marsBorrowRateAfter = yield queryBorrowRate(terra, redBank, { cw20: { contract_addr: mars } });
        // the new rate should be higher than the previous rate because the new utilization rate is higher
        // than the optimal utilization rate
        assert(marsBorrowRateAfter > marsBorrowRateBefore);
        console.log('bob repays mars');
        marsBorrowRateBefore = marsBorrowRateAfter;
        yield executeContract(terra, bob, mars, {
            send: {
                contract: redBank,
                amount: String(0.8 * ALICE_MARS_COLLATERAL),
                msg: toEncodedBinary({ repay_cw20: {} }),
            },
        }, { logger: logger });
        marsBorrowRateAfter = yield queryBorrowRate(terra, redBank, { cw20: { contract_addr: mars } });
        // the new rate should be lower than the previous rate because the new utilization rate is lower
        // than the optimal utilization rate
        assert(marsBorrowRateAfter < marsBorrowRateBefore);
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    // addresses
    const deployer = terra.wallets.test1;
    const alice = terra.wallets.test2;
    const bob = terra.wallets.test3;
    // mock contract addresses
    const protocolRewardsCollector = new MnemonicKey().accAddress;
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
    const mars = yield deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, 'cw20_base.wasm'), {
        name: 'Mars',
        symbol: 'MARS',
        decimals: 6,
        initial_balances: [
            { address: alice.key.accAddress, amount: String(100 * ALICE_MARS_COLLATERAL) },
            { address: bob.key.accAddress, amount: String(100 * BOB_MARS_COLLATERAL) },
        ],
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
                protocol_admin_address: deployer.key.accAddress,
            },
        },
    }, { logger: logger });
    console.log('init assets');
    // mars
    yield executeContract(terra, deployer, redBank, {
        init_asset: {
            asset: { cw20: { contract_addr: mars } },
            asset_params: {
                initial_borrow_rate: '0.1',
                max_loan_to_value: String(MARS_MAX_LTV),
                reserve_factor: '0',
                liquidation_threshold: '0.65',
                liquidation_bonus: '0.1',
                interest_rate_model_params: {
                    dynamic: {
                        min_borrow_rate: '0.0',
                        max_borrow_rate: '2.0',
                        kp_1: '0.02',
                        optimal_utilization_rate: String(MARS_OPTIMAL_UTILIZATION_RATE),
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
    yield executeContract(terra, deployer, oracle, {
        set_asset: {
            asset: { cw20: { contract_addr: mars } },
            price_source: { fixed: { price: String(MARS_USD_PRICE) } },
        },
    }, { logger: logger });
    const maMars = yield queryMaAssetAddress(terra, redBank, { cw20: { contract_addr: mars } });
    // uusd
    yield executeContract(terra, deployer, redBank, {
        init_asset: {
            asset: { native: { denom: 'uusd' } },
            asset_params: {
                initial_borrow_rate: '0.2',
                max_loan_to_value: String(UUSD_MAX_LTV),
                reserve_factor: '0',
                liquidation_threshold: '0.85',
                liquidation_bonus: '0.1',
                interest_rate_model_params: {
                    linear: {
                        optimal_utilization_rate: '1',
                        base: '0',
                        slope_1: String(UUSD_LINEAR_INTEREST_RATE_SLOPE_1),
                        slope_2: '0',
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    yield executeContract(terra, deployer, oracle, {
        set_asset: {
            asset: { native: { denom: 'uusd' } },
            price_source: { fixed: { price: '1' } },
        },
    }, { logger: logger });
    const maUusd = yield queryMaAssetAddress(terra, redBank, { native: { denom: 'uusd' } });
    const env = { terra, redBank, mars, maUusd, maMars, alice, bob };
    console.log('testLinearInterestRate');
    yield testLinearInterestRate(env, logger);
    console.log('testDynamicInterestRate');
    yield testDynamicInterestRate(env, logger);
    console.log('OK');
    logger.showGasConsumption();
}))();
