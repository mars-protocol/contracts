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
import { join } from 'path';
import 'dotenv/config.js';
import { deployContract, executeContract, Logger, setTimeoutDuration, uploadContract } from '../helpers.js';
import { borrowNative, depositCw20, depositNative, setAssetOraclePriceSource } from './test_helpers.js';
// CONSTS
// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH;
const UUSD_COLLATERAL = 1000000000000;
const MARS_COLLATERAL = 100000000000000;
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    const deployer = terra.wallets.test1;
    const provider = terra.wallets.test2;
    const borrower = terra.wallets.test3;
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
        initial_balances: [{ address: borrower.key.accAddress, amount: String(MARS_COLLATERAL) }],
    });
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
                max_loan_to_value: '0.55',
                reserve_factor: '0.2',
                liquidation_threshold: '0.65',
                liquidation_bonus: '0.1',
                interest_rate_model_params: {
                    linear: {
                        optimal_utilization_rate: '1',
                        base: '0',
                        slope_1: '1',
                        slope_2: '0',
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    yield setAssetOraclePriceSource(terra, deployer, oracle, { cw20: { contract_addr: mars } }, 2, logger);
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
                    linear: {
                        optimal_utilization_rate: '1',
                        base: '0',
                        slope_1: '1',
                        slope_2: '0',
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
    console.log('provide uusd');
    yield depositNative(terra, provider, redBank, 'uusd', UUSD_COLLATERAL, logger);
    console.log('provide mars');
    yield depositCw20(terra, borrower, redBank, mars, MARS_COLLATERAL, logger);
    console.log('borrow uusd');
    yield borrowNative(terra, borrower, redBank, 'uusd', UUSD_COLLATERAL, logger);
    console.log('OK');
    logger.showGasConsumption();
}))();
