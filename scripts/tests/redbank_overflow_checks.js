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
import { deployContract, executeContract, Logger, queryContract, setGasAdjustment, setTimeoutDuration, uploadContract, } from '../helpers.js';
import { borrowNative, depositNative, queryBalanceNative, setAssetOraclePriceSource } from './test_helpers.js';
// CONSTS
// Max available uusd funds in LocalTerra wallet
const UUSD_MAX_BALANCE = 10000000000000000;
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    // gas is not correctly estimated in the repay_native method on the red bank,
    // so any estimates need to be adjusted upwards
    setGasAdjustment(2);
    const logger = new Logger();
    const terra = new LocalTerra();
    const deployer = terra.wallets.test1;
    const user = terra.wallets.test2;
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
    // uusd
    const maxLTV = 0.99;
    yield executeContract(terra, deployer, redBank, {
        init_asset: {
            asset: { native: { denom: 'uusd' } },
            asset_params: {
                initial_borrow_rate: '0.01',
                max_loan_to_value: maxLTV.toString(),
                reserve_factor: '0.01',
                liquidation_threshold: '1.0',
                liquidation_bonus: '0.01',
                interest_rate_model_params: {
                    dynamic: {
                        min_borrow_rate: '0.0',
                        max_borrow_rate: '1.0',
                        kp_1: '0.04',
                        optimal_utilization_rate: '0.8',
                        kp_augmentation_threshold: '0.2',
                        kp_2: '0.08',
                        update_threshold_txs: 1,
                        update_threshold_seconds: 1,
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    yield setAssetOraclePriceSource(terra, deployer, oracle, { native: { denom: 'uusd' } }, 1, logger);
    // TESTS
    const uusdBalance = yield queryBalanceNative(terra, user.key.accAddress, 'uusd');
    console.log('User balance in uusd:', uusdBalance);
    // Leave some UST in the wallet for fee (for example: deposit and repay transactions)
    const depositAmount = UUSD_MAX_BALANCE - 100000000;
    console.log('Deposit max uusd available (minus fee):', depositAmount);
    yield depositNative(terra, user, redBank, 'uusd', depositAmount, logger);
    const userPositionT1 = yield queryContract(terra, redBank, { user_position: { user_address: user.key.accAddress } });
    strictEqual(Number(userPositionT1.total_collateral_in_uusd), depositAmount);
    const borrowAmount = maxLTV * depositAmount;
    console.log('Borrow max uusd:', borrowAmount);
    yield borrowNative(terra, user, redBank, 'uusd', borrowAmount, logger);
    const userPositionT2 = yield queryContract(terra, redBank, { user_position: { user_address: user.key.accAddress } });
    const totalDebtInUusd = Number(userPositionT2.total_debt_in_uusd);
    strictEqual(totalDebtInUusd, borrowAmount);
    // hack: Just do a big number to repay all debt
    const repayAmount = totalDebtInUusd + 10000000;
    console.log('Repay max borrowed uusd:', repayAmount);
    yield executeContract(terra, user, redBank, { repay_native: { denom: 'uusd' } }, { coins: `${repayAmount}uusd`, logger: logger });
    const userPositionT3 = yield queryContract(terra, redBank, { user_position: { user_address: user.key.accAddress } });
    strictEqual(Number(userPositionT3.total_debt_in_uusd), 0);
    console.log('Withdraw max uusd');
    yield executeContract(terra, user, redBank, { withdraw: { asset: { native: { denom: 'uusd' } } } }, { logger: logger });
    const userPositionT4 = yield queryContract(terra, redBank, { user_position: { user_address: user.key.accAddress } });
    strictEqual(Number(userPositionT4.total_collateral_in_uusd), 0);
    console.log('OK');
    logger.showGasConsumption();
}))();
