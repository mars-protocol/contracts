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
import { Dec, LocalTerra } from '@terra-money/terra.js';
import { strictEqual } from 'assert';
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, sleep } from '../helpers.js';
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
// TESTS
function testLunaPrice(terra, deployer, oracle, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        console.log('testLunaPrice');
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom: 'uluna' } },
                price_source: { native: { denom: 'uluna' } },
            },
        }, { logger: logger });
        const marsOraclePrice = yield queryContract(terra, oracle, {
            asset_price: { asset: { native: { denom: 'uluna' } } },
        });
        const terraOraclePrice = yield terra.oracle.exchangeRate('uusd');
        strictEqual(new Dec(marsOraclePrice).toString(), terraOraclePrice === null || terraOraclePrice === void 0 ? void 0 : terraOraclePrice.amount.toString());
    });
}
function testNativeTokenPrice(terra, deployer, oracle, denom, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        console.log('testNativeTokenPrice:', denom);
        yield executeContract(terra, deployer, oracle, {
            set_asset: {
                asset: { native: { denom } },
                price_source: { native: { denom } },
            },
        }, { logger: logger });
        const marsOraclePrice = yield queryContract(terra, oracle, { asset_price: { asset: { native: { denom } } } });
        const terraOraclePrice = yield terra.oracle.exchangeRate(denom);
        const terraOracleLunaUsdPrice = yield terra.oracle.exchangeRate('uusd');
        const denomUsdPrice = new Dec(terraOracleLunaUsdPrice === null || terraOracleLunaUsdPrice === void 0 ? void 0 : terraOracleLunaUsdPrice.amount).div(new Dec(terraOraclePrice === null || terraOraclePrice === void 0 ? void 0 : terraOraclePrice.amount));
        strictEqual(new Dec(marsOraclePrice).toString(), denomUsdPrice.toString());
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    const deployer = terra.wallets.test1;
    yield waitUntilTerraOracleAvailable(terra);
    console.log('upload contracts');
    const oracle = yield deployContract(terra, deployer, '../artifacts/mars_oracle.wasm', {
        owner: deployer.key.accAddress,
    });
    yield testLunaPrice(terra, deployer, oracle);
    yield testNativeTokenPrice(terra, deployer, oracle, 'uusd', logger);
    yield testNativeTokenPrice(terra, deployer, oracle, 'ueur', logger);
    yield testNativeTokenPrice(terra, deployer, oracle, 'ukrw', logger);
    console.log('OK');
    logger.showGasConsumption();
}))();
