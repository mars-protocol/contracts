/*
LocalTerra requires >= 1500 ms block times for the native Terra oracle to work:

```
sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/1500ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
```
*/
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { LocalTerra } from '@terra-money/terra.js';
import { strictEqual } from 'assert';
import { join } from 'path';
import 'dotenv/config.js';
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, setGasAdjustment, sleep, uploadContract, } from '../helpers.js';
import { approximateEqual, depositNative } from './test_helpers.js';
// CONSTS
// required environment variables:
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH;
// HELPERS
function waitUntilTerraOracleAvailable(terra) {
    return __awaiter(this, void 0, void 0, function* () {
        let tries = 0;
        const maxTries = 10;
        let backoff = 1;
        while (true) {
            const activeDenoms = yield terra.oracle.activeDenoms();
            if (activeDenoms.includes('uusd')) {
                break;
            }
            // timeout
            tries++;
            if (tries == maxTries) {
                throw new Error(`Terra oracle not available after ${maxTries} tries`);
            }
            // exponential backoff
            console.log(`Terra oracle not available, sleeping for ${backoff} s`);
            yield sleep(backoff * 1000);
            backoff *= 2;
        }
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    setGasAdjustment(2);
    const logger = new Logger();
    const terra = new LocalTerra();
    yield waitUntilTerraOracleAvailable(terra);
    // addresses
    const deployer = terra.wallets.test1;
    // mock contract addresses
    const astroportGenerator = terra.wallets.test9.key.accAddress;
    const protocolRewardsCollector = terra.wallets.test10.key.accAddress;
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
                        update_threshold_seconds: 1,
                    },
                },
                active: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
        },
    }, { logger: logger });
    console.log('setup astroport pair');
    const tokenCodeID = yield uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, 'astroport_token.wasm'));
    const pairCodeID = yield uploadContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, 'astroport_pair.wasm'));
    const astroportFactory = yield deployContract(terra, deployer, join(ASTROPORT_ARTIFACTS_PATH, 'astroport_factory.wasm'), {
        owner: deployer.key.accAddress,
        token_code_id: tokenCodeID,
        generator_address: astroportGenerator,
        pair_configs: [
            {
                code_id: pairCodeID,
                pair_type: { xyk: {} },
                total_fee_bps: 0,
                maker_fee_bps: 0,
            },
        ],
    });
    let result = yield executeContract(terra, deployer, astroportFactory, {
        create_pair: {
            pair_type: { xyk: {} },
            asset_infos: [{ native_token: { denom: 'uluna' } }, { native_token: { denom: 'uusd' } }],
        },
    }, { logger: logger });
    const ulunaUusdPair = result.logs[0].eventsByType.wasm.pair_contract_addr[0];
    // TESTS
    console.log('test oracle price sources');
    {
        console.log('- fixed');
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom: 'uluna' } },
                price_source: { fixed: { price: '25' } },
            },
        }, { logger: logger });
        const alice = terra.wallets.test2;
        yield depositNative(terra, alice, redBank, 'uluna', 1000000, logger);
        const userPosition = yield queryContract(terra, redBank, { user_position: { user_address: alice.key.accAddress } });
        // 1 luna should be worth $25
        strictEqual(parseInt(userPosition.total_collateral_in_uusd), 25000000);
    }
    {
        console.log('- astroport spot');
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom: 'uluna' } },
                price_source: { astroport_spot: { pair_address: ulunaUusdPair } },
            },
        }, { logger: logger });
        const bob = terra.wallets.test3;
        yield depositNative(terra, bob, redBank, 'uluna', 1000000, logger);
        // provide liquidity such that the price of luna is $30
        yield executeContract(terra, deployer, ulunaUusdPair, {
            provide_liquidity: {
                assets: [
                    {
                        info: { native_token: { denom: 'uluna' } },
                        amount: String(1000000000000),
                    },
                    {
                        info: { native_token: { denom: 'uusd' } },
                        amount: String(30000000000000),
                    },
                ],
            },
        }, { coins: `1000000000000uluna,30000000000000uusd`, logger: logger });
        const userPosition = yield queryContract(terra, redBank, { user_position: { user_address: bob.key.accAddress } });
        // 1 luna should be worth $30
        approximateEqual(parseInt(userPosition.total_collateral_in_uusd), 30000000, 100);
    }
    {
        console.log('- astroport twap');
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom: 'uluna' } },
                price_source: {
                    astroport_twap: {
                        pair_address: ulunaUusdPair,
                        window_size: 2,
                        tolerance: 1,
                    },
                },
            },
        }, { logger: logger });
        const carol = terra.wallets.test4;
        yield depositNative(terra, carol, redBank, 'uluna', 1000000, logger);
        // trigger cumulative prices to be updated
        yield executeContract(terra, deployer, ulunaUusdPair, {
            provide_liquidity: {
                assets: [
                    {
                        info: { native_token: { denom: 'uluna' } },
                        amount: String(1),
                    },
                    {
                        info: { native_token: { denom: 'uusd' } },
                        amount: String(30),
                    },
                ],
            },
        }, { coins: `1uluna,30uusd`, logger: logger });
        // record TWAP
        yield executeContract(terra, deployer, oracle, { record_twap_snapshots: { assets: [{ native: { denom: 'uluna' } }] } }, { logger: logger });
        // wait until a twap snapshot can be recorded again
        yield sleep(1500);
        // record TWAP
        yield executeContract(terra, deployer, oracle, { record_twap_snapshots: { assets: [{ native: { denom: 'uluna' } }] } }, { logger: logger });
        const userPosition = yield queryContract(terra, redBank, { user_position: { user_address: carol.key.accAddress } });
        // 1 luna should be worth $30
        strictEqual(parseInt(userPosition.total_collateral_in_uusd), 30000000);
    }
    {
        console.log('- native');
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom: 'uluna' } },
                price_source: { native: { denom: 'uluna' } },
            },
        }, { logger: logger });
        const dan = terra.wallets.test5;
        yield depositNative(terra, dan, redBank, 'uluna', 1000000, logger);
        const userPosition = yield queryContract(terra, redBank, { user_position: { user_address: dan.key.accAddress } });
        const lunaUsdPrice = yield terra.oracle.exchangeRate('uusd');
        const lunaUusdPrice = lunaUsdPrice === null || lunaUsdPrice === void 0 ? void 0 : lunaUsdPrice.amount.mul(1000000).floor().toNumber();
        strictEqual(parseInt(userPosition.total_collateral_in_uusd), lunaUusdPrice);
    }
    console.log('OK');
    logger.showGasConsumption();
}))();
