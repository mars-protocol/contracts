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
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, uploadContract, } from '../helpers.js';
import { strict as assert } from 'assert';
import { borrowNative, depositNative, queryMaAssetAddress, setAssetOraclePriceSource, transferCw20, } from './test_helpers.js';
// CONSTS
const USD_COLLATERAL = 100000000000;
const LUNA_COLLATERAL = 100000000000;
const USD_BORROW = 100000000000;
const MA_TOKEN_SCALING_FACTOR = 1000000;
// HELPERS
function checkCollateral(terra, wallet, redBank, denom, enabled) {
    return __awaiter(this, void 0, void 0, function* () {
        const collateral = yield queryContract(terra, redBank, { user_collateral: { user_address: wallet.key.accAddress } });
        for (const c of collateral.collateral) {
            if (c.denom == denom && c.enabled == enabled) {
                return true;
            }
        }
        return false;
    });
}
// TESTS
function testHealthFactorChecks(terra, redBank, maLuna, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const provider = terra.wallets.test2;
        const borrower = terra.wallets.test3;
        const recipient = terra.wallets.test4;
        console.log('provider provides USD');
        yield depositNative(terra, provider, redBank, 'uusd', USD_COLLATERAL, logger);
        console.log('borrower provides Luna');
        yield depositNative(terra, borrower, redBank, 'uluna', LUNA_COLLATERAL, logger);
        console.log('borrower borrows USD');
        yield borrowNative(terra, borrower, redBank, 'uusd', USD_BORROW, logger);
        console.log('transferring the entire maToken balance should fail');
        yield assert.rejects(transferCw20(terra, borrower, maLuna, recipient.key.accAddress, LUNA_COLLATERAL * MA_TOKEN_SCALING_FACTOR, logger), (error) => {
            return error.response.data.message.includes('Cannot make token transfer if it results in a health factor lower than 1 for the sender');
        });
        console.log('transferring a small amount of the maToken balance should work');
        assert(yield checkCollateral(terra, recipient, redBank, 'uluna', false));
        yield transferCw20(terra, borrower, maLuna, recipient.key.accAddress, Math.floor((LUNA_COLLATERAL * MA_TOKEN_SCALING_FACTOR) / 100), logger);
        assert(yield checkCollateral(terra, recipient, redBank, 'uluna', true));
    });
}
function testCollateralStatusChanges(terra, redBank, maLuna, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const provider = terra.wallets.test5;
        const recipient = terra.wallets.test6;
        console.log('provider provides Luna');
        yield depositNative(terra, provider, redBank, 'uluna', LUNA_COLLATERAL, logger);
        assert(yield checkCollateral(terra, provider, redBank, 'uluna', true));
        assert(yield checkCollateral(terra, recipient, redBank, 'uluna', false));
        console.log('transferring all maTokens to recipient should enable that asset as collateral');
        yield transferCw20(terra, provider, maLuna, recipient.key.accAddress, LUNA_COLLATERAL * MA_TOKEN_SCALING_FACTOR, logger);
        assert(yield checkCollateral(terra, provider, redBank, 'uluna', false));
        assert(yield checkCollateral(terra, recipient, redBank, 'uluna', true));
    });
}
function testTransferCollateral(terra, redBank, maLuna, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        const provider = terra.wallets.test7;
        const borrower = terra.wallets.test8;
        const recipient = terra.wallets.test9;
        console.log('provider provides USD');
        yield depositNative(terra, provider, redBank, 'uusd', USD_COLLATERAL, logger);
        console.log('borrower provides Luna');
        yield depositNative(terra, borrower, redBank, 'uluna', LUNA_COLLATERAL, logger);
        console.log('borrower borrows USD');
        yield borrowNative(terra, borrower, redBank, 'uusd', USD_COLLATERAL / 100, logger);
        console.log('disabling Luna as collateral should fail');
        assert(yield checkCollateral(terra, borrower, redBank, 'uluna', true));
        yield assert.rejects(executeContract(terra, borrower, redBank, {
            update_asset_collateral_status: {
                asset: { native: { denom: 'uluna' } },
                enable: false,
            },
        }, { logger: logger }), (error) => {
            return error.response.data.message.includes("User's health factor can't be less than 1 after disabling collateral");
        });
        console.log('transfer maLuna');
        yield transferCw20(terra, borrower, maLuna, recipient.key.accAddress, Math.floor((LUNA_COLLATERAL * MA_TOKEN_SCALING_FACTOR) / 100), logger);
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    // addresses
    const deployer = terra.wallets.test1;
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
    yield executeContract(terra, deployer, addressProvider, {
        update_config: {
            config: {
                owner: deployer.key.accAddress,
                incentives_address: incentives,
                oracle_address: oracle,
                red_bank_address: redBank,
                protocol_rewards_collector_address: protocolRewardsCollector,
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
                        update_threshold_txs: 5,
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
                        update_threshold_txs: 5,
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
    const maLuna = yield queryMaAssetAddress(terra, redBank, { native: { denom: 'uluna' } });
    // tests
    console.log('testHealthFactorChecks');
    yield testHealthFactorChecks(terra, redBank, maLuna, logger);
    console.log('testCollateralStatusChanges');
    yield testCollateralStatusChanges(terra, redBank, maLuna, logger);
    console.log('testTransferCollateral');
    yield testTransferCollateral(terra, redBank, maLuna, logger);
    console.log('OK');
    logger.showGasConsumption();
}))();
