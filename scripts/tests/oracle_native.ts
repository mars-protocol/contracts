/*
LocalTerra requires >= 1500 ms block times for the native Terra oracle to work:

```
sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/1500ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
```
*/

import { Dec, LCDClient, LocalTerra, Wallet } from '@terra-money/terra.js';
import { strictEqual } from 'assert';
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, sleep } from '../helpers.js';

// HELPERS

async function waitUntilTerraOracleAvailable(terra: LCDClient) {
  let tries = 0;
  const maxTries = 10;
  let backoff = 1;
  while (true) {
    const activeDenoms = await terra.oracle.activeDenoms();
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
    await sleep(backoff * 1000);
    backoff *= 2;
  }
}

// TESTS

async function testLunaPrice(terra: LCDClient, deployer: Wallet, oracle: string, logger?: Logger) {
  console.log('testLunaPrice');

  await executeContract(
    terra,
    deployer,
    oracle,
    {
      set_asset: {
        asset: { native: { denom: 'uluna' } },
        price_source: { native: { denom: 'uluna' } },
      },
    },
    { logger: logger },
  );

  const marsOraclePrice = await queryContract(terra, oracle, {
    asset_price: { asset: { native: { denom: 'uluna' } } },
  });
  const terraOraclePrice = await terra.oracle.exchangeRate('uusd');

  strictEqual(new Dec(marsOraclePrice).toString(), terraOraclePrice?.amount.toString());
}

async function testNativeTokenPrice(
  terra: LCDClient,
  deployer: Wallet,
  oracle: string,
  denom: string,
  logger?: Logger,
) {
  console.log('testNativeTokenPrice:', denom);

  await executeContract(
    terra,
    deployer,
    oracle,
    {
      set_asset: {
        asset: { native: { denom } },
        price_source: { native: { denom } },
      },
    },
    { logger: logger },
  );

  const marsOraclePrice = await queryContract(terra, oracle, { asset_price: { asset: { native: { denom } } } });
  const terraOraclePrice = await terra.oracle.exchangeRate(denom);
  const terraOracleLunaUsdPrice = await terra.oracle.exchangeRate('uusd');

  const denomUsdPrice = new Dec(terraOracleLunaUsdPrice?.amount).div(new Dec(terraOraclePrice?.amount));

  strictEqual(new Dec(marsOraclePrice).toString(), denomUsdPrice.toString());
}

// MAIN

(async () => {
  setTimeoutDuration(0);

  const logger = new Logger();

  const terra = new LocalTerra();
  const deployer = terra.wallets.test1;

  await waitUntilTerraOracleAvailable(terra);

  console.log('upload contracts');

  const oracle = await deployContract(terra, deployer, '../artifacts/mars_oracle.wasm', {
    owner: deployer.key.accAddress,
  });

  await testLunaPrice(terra, deployer, oracle);

  await testNativeTokenPrice(terra, deployer, oracle, 'uusd', logger);
  await testNativeTokenPrice(terra, deployer, oracle, 'ueur', logger);
  await testNativeTokenPrice(terra, deployer, oracle, 'ukrw', logger);

  console.log('OK');

  logger.showGasConsumption();
})();
