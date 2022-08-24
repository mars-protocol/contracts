import {
  ExecuteResult,
  InstantiateResult,
  SigningCosmWasmClient,
  UploadResult,
  SigningCosmWasmClientOptions,
} from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, Coin } from '@cosmjs/stargate';
import 'dotenv/config.js';
import { Event } from '@cosmjs/stargate/build/logs';
import * as fs from 'fs';
import { readArtifact, writeArtifact } from './deploy_helpers.js';

// TO RUN, SET THE DESIRED NETWORK BELOW AND RUN
// npm run deploy

requiredEnvironmentVariables([
  'NETWORK_TO_DEPLOY_TO',
  'MARS_DENOM',
  'OSMOSIS_DENOM',
  'CHAIN_PREFIX',
  'SAFETY_FUND_FEE_SHARE',
  'CHANNEL_ID',
  'TIMEOUT_REVISION',
  'REWARD_COLLECTOR_TIMEOUT_BLOCKS',
  'REWARD_COLLECTOR_TIMEOUT_SECONDS',
]);

// CONSTANTS
const NETWORK_TO_DEPLOY_TO = process.env.NETWORK_TO_DEPLOY_TO!;
const MARS_DENOM = process.env.MARS_DENOM!;
const OSMOSIS_DENOM = process.env.OSMOSIS_DENOM!;
const ATOM_DENOM = process.env.ATOM_DENOM!;
const PREFIX = process.env.CHAIN_PREFIX!;
const SAFETY_FUND_FEE_SHARE = process.env.SAFETY_FUND_FEE_SHARE!;
const SAFETY_FUND_DENOM = OSMOSIS_DENOM;
const FEE_COLLECTOR_DENOM = OSMOSIS_DENOM;
const CHANNEL_ID = process.env.CHANNEL_ID!;
const TIMEOUT_REVISION = process.env.TIMEOUT_REVISION!;
const REWARD_COLLECTOR_TIMEOUT_BLOCKS = process.env.REWARD_COLLECTOR_TIMEOUT_BLOCKS!;
const REWARD_COLLECTOR_TIMEOUT_SECONDS = process.env.REWARD_COLLECTOR_TIMEOUT_SECONDS!;

let chain_id: string;
let rpc_endpoint: string;
let deployer_wallet_mnemonic: string;

async function main() {
  if (NETWORK_TO_DEPLOY_TO === 'localosmosis') {
    chain_id = 'localosmosis';
    rpc_endpoint = 'http://localhost:26657';
    deployer_wallet_mnemonic =
      'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius';
  } else if (NETWORK_TO_DEPLOY_TO === 'osmo-test-4') {
    chain_id = 'osmo-test-4';
    rpc_endpoint = 'https://rpc-test.osmosis.zone';
    deployer_wallet_mnemonic =
      'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process';
  } else {
    console.error(
      `ERROR: Currently these scripts are intended for LocalOsmosis and 'osmo-test-4' testnet environents only, ${NETWORK_TO_DEPLOY_TO} deployment not yet supported`,
    );
    process.exit(1);
  }

  console.log(`deploying to chain id: ${chain_id}`);

  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(deployer_wallet_mnemonic, { prefix: PREFIX });

  const accounts = await wallet.getAccounts();
  const deployerAddress = accounts[0].address;
  console.log('deployer address is: ', deployerAddress);

  const clientOption: SigningCosmWasmClientOptions = {
    gasPrice: GasPrice.fromString('0.1uosmo'),
  };
  const client = await SigningCosmWasmClient.connectWithSigner(rpc_endpoint, wallet, clientOption);

  const accountBalance = await client.getBalance(deployerAddress, OSMOSIS_DENOM);
  console.log(`uosmo account balance is: ${accountBalance.amount} (${Number(accountBalance.amount) / 1e6} OSMO)`);
  if (Number(accountBalance.amount) < 1_000000 && chain_id === 'osmo-test-4') {
    console.log('get more OSMO tokens at: https://faucet.osmosis.zone/#/');
    return;
  }

  const ARTIFACTS_PATH = '../artifacts/';

  // network : stores contract addresses
  const network = readArtifact(chain_id);
  console.log('network:', network);

  let uploadResult: UploadResult;
  let result: InstantiateResult | ExecuteResult;
  let msg: Record<string, unknown>;
  let wasmEvent: Event | undefined;
  let coins: Coin[];

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD RED BANK CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.redBankCodeId) {
    const redBankWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_red_bank.wasm');
    uploadResult = await client.upload(deployerAddress, redBankWasm, 'auto');
    network.redBankCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Red Bank Code ID : ${network.redBankCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD ADDRESS PROVIDER CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.addressProviderCodeId) {
    const addressProviderWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_address_provider.wasm');
    uploadResult = await client.upload(deployerAddress, addressProviderWasm, 'auto');
    network.addressProviderCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Address Provider Code ID : ${network.addressProviderCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD MA TOKEN CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.maTokenCodeId) {
    const maTokenWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_ma_token.wasm');
    uploadResult = await client.upload(deployerAddress, maTokenWasm, 'auto');
    network.maTokenCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: maToken Code ID : ${network.maTokenCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD INCENTIVES CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.incentivesCodeId) {
    const incentivesWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_incentives.wasm');
    uploadResult = await client.upload(deployerAddress, incentivesWasm, 'auto');
    network.incentivesCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Incentives Code ID : ${network.incentivesCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD ORACLE CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.oracleCodeId) {
    const oracleWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_oracle_osmosis.wasm');
    uploadResult = await client.upload(deployerAddress, oracleWasm, 'auto');
    network.oracleCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Oracle Code ID : ${network.oracleCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP UPLOAD PROTOCOL REWARDS COLLECTOR CONTRACT
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.protocolRewardsCollectorCodeId) {
    const protocolRewardsCollectorWasm = fs.readFileSync(ARTIFACTS_PATH + 'mars_rewards_collector_osmosis.wasm');
    uploadResult = await client.upload(deployerAddress, protocolRewardsCollectorWasm, 'auto');
    network.protocolRewardsCollectorCodeId = uploadResult.codeId;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Protocol Rewards Collector Code ID : ${network.protocolRewardsCollectorCodeId}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // UPLOADS FINISHED
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP INITIALISE ADDRESS PROVIDER
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.addressProviderContractAddress) {
    msg = { owner: deployerAddress, prefix: PREFIX };
    const { contractAddress: addressProviderContractAddress } = await client.instantiate(
      deployerAddress,
      network.addressProviderCodeId,
      msg,
      'mars-address-provider',
      'auto',
    );
    network.addressProviderContractAddress = addressProviderContractAddress;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Address Provider Contract Address : ${network.addressProviderContractAddress}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP INITIALISE RED BANK
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.redBankContractAddress) {
    msg = {
      config: {
        owner: deployerAddress,
        address_provider_address: network.addressProviderContractAddress,
        ma_token_code_id: network.maTokenCodeId,
        close_factor: '0.5',
      },
    };
    const { contractAddress: redBankContractAddress } = await client.instantiate(
      deployerAddress,
      network.redBankCodeId,
      msg,
      'mars-red-bank',
      'auto',
    );
    network.redBankContractAddress = redBankContractAddress;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Red Bank Contract Address : ${network.redBankContractAddress}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP INITIALISE INCENTIVES
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.incentivesContractAddress) {
    msg = {
      owner: deployerAddress,
      address_provider_address: network.addressProviderContractAddress,
      mars_denom: MARS_DENOM,
    };
    const { contractAddress: incentivesContractAddress } = await client.instantiate(
      deployerAddress,
      network.incentivesCodeId,
      msg,
      'mars-incentives',
      'auto',
    );
    network.incentivesContractAddress = incentivesContractAddress;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Incentives Contract Address : ${network.incentivesContractAddress}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP INITIALISE ORACLE
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.oracleContractAddress) {
    msg = { owner: deployerAddress, base_denom: OSMOSIS_DENOM };
    const { contractAddress: oracleContractAddress } = await client.instantiate(
      deployerAddress,
      network.oracleCodeId,
      msg,
      'mars-oracle',
      'auto',
    );
    network.oracleContractAddress = oracleContractAddress;
    writeArtifact(network, chain_id);
  }
  console.log(`${chain_id} :: Oracle Contract Address : ${network.oracleContractAddress}`);

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP INITIALISE PROTOCOL REWARDS COLLECTOR
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.protocolRewardsCollectorContractAddress) {
    msg = {
      owner: deployerAddress,
      prefix: String(PREFIX),
      address_provider: network.addressProviderContractAddress,
      safety_tax_rate: String(SAFETY_FUND_FEE_SHARE),
      safety_fund_denom: String(SAFETY_FUND_DENOM),
      fee_collector_denom: String(FEE_COLLECTOR_DENOM),
      channel_id: String(CHANNEL_ID),
      timeout_revision: Number(TIMEOUT_REVISION),
      timeout_blocks: Number(REWARD_COLLECTOR_TIMEOUT_BLOCKS),
      timeout_seconds: Number(REWARD_COLLECTOR_TIMEOUT_SECONDS),
    };

    const { contractAddress: protocolRewardsCollectorContractAddress } = await client.instantiate(
      deployerAddress,
      network.protocolRewardsCollectorCodeId,
      msg,
      'mars-protocol-rewards-collector',
      'auto',
    );
    network.protocolRewardsCollectorContractAddress = protocolRewardsCollectorContractAddress;

    // set osmo : atom route
    // todo search for correct pool and create if does not exist.
    await client.execute(
      deployerAddress,
      network.protocolRewardsCollectorContractAddress,
      {
        set_route: {
          denom_in: OSMOSIS_DENOM,
          denom_out: ATOM_DENOM,
          route: [{ denom_out: ATOM_DENOM, pool_id: 1 }],
        },
      },
      'auto',
    );

    writeArtifact(network, chain_id);
  }
  console.log(
    `${chain_id} :: Protocol Rewards Collector Contract Address : ${network.protocolRewardsCollectorContractAddress}`,
  );

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP EXECUTE UPDATE ADDRESS PROVIDER CONFIGURATION
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.addressProviderUpdated) {
    const addressesToSet = [
      {
        contract: 'protocol_rewards_collector',
        address: network.protocolRewardsCollectorContractAddress,
      },
      {
        contract: 'incentives',
        address: network.incentivesContractAddress,
      },
      {
        contract: 'oracle',
        address: network.oracleContractAddress,
      },
      {
        contract: 'protocol_admin',
        address: deployerAddress,
      },
      {
        contract: 'red_bank',
        address: network.redBankContractAddress,
      },
    ];

    // When executeMultiple is released to npm, switch to that
    for (const index in addressesToSet) {
      const addressObject = addressesToSet[index];
      result = await client.execute(
        deployerAddress,
        network.addressProviderContractAddress,
        { set_address: addressObject },
        'auto',
      );

      wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
      console.info('The `wasm` event emitted by the contract execution:', wasmEvent);
    }

    network.addressProviderUpdated = true;
    writeArtifact(network, chain_id);
  }
  console.log(
    `${chain_id} :: Address Provider config : `,
    await client.queryContractSmart(network.addressProviderContractAddress, { config: {} }),
  );

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP EXECUTE INITIALISE RED BANK ASSET
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.uosmoRedBankMarketInitialised) {
    msg = {
      init_asset: {
        denom: OSMOSIS_DENOM,
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
              optimal_utilization_rate: '0.7',
              kp_1: '0.02',
              kp_2: '0.05',
              kp_augmentation_threshold: '0.15',
              update_threshold_txs: 10,
              update_threshold_seconds: 3600,
            },
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true,
        },
        asset_symbol: 'OSMO',
      },
    };

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);
    network.uosmoRedBankMarketInitialised = true;
    writeArtifact(network, chain_id);
  }
  console.log(
    `${chain_id} :: Red Bank config : `,
    await client.queryContractSmart(network.redBankContractAddress, { config: {} }),
  );
  console.log(
    `${chain_id} :: Red Bank markets list : `,
    await client.queryContractSmart(network.redBankContractAddress, { markets: {} }),
  );
  console.log(
    `${chain_id} :: Red Bank uosmo market config : `,
    await client.queryContractSmart(network.redBankContractAddress, { market: { denom: OSMOSIS_DENOM } }),
  );

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP EXECUTE INITIALISE RED BANK ASSET
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.uatomRedBankMarketInitialised) {
    msg = {
      init_asset: {
        denom: ATOM_DENOM,
        asset_params: {
          initial_borrow_rate: '0.1',
          max_loan_to_value: '0.65',
          reserve_factor: '0.2',
          liquidation_threshold: '0.7',
          liquidation_bonus: '0.1',
          interest_rate_model_params: {
            dynamic: {
              min_borrow_rate: '0.0',
              max_borrow_rate: '2.0',
              optimal_utilization_rate: '0.7',
              kp_1: '0.02',
              kp_2: '0.05',
              kp_augmentation_threshold: '0.15',
              update_threshold_txs: 10,
              update_threshold_seconds: 3600,
            },
          },
          active: true,
          deposit_enabled: true,
          borrow_enabled: true,
        },
        asset_symbol: 'ATOM',
      },
    };

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);
    network.uatomRedBankMarketInitialised = true;
    writeArtifact(network, chain_id);
  }
  console.log(
    `${chain_id} :: Red Bank config : `,
    await client.queryContractSmart(network.redBankContractAddress, { config: {} }),
  );
  console.log(
    `${chain_id} :: Red Bank markets list : `,
    await client.queryContractSmart(network.redBankContractAddress, { markets: {} }),
  );
  console.log(
    `${chain_id} :: Red Bank uatom market config : `,
    await client.queryContractSmart(network.redBankContractAddress, {
      market: { denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2' },
    }),
  );

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP EXECUTE SET ORACLE ASSET UOSMO
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.uosmoOraclePriceSet) {
    msg = {
      set_price_source: {
        denom: OSMOSIS_DENOM,
        price_source: {
          fixed: { price: '1.0' },
        },
      },
    };

    result = await client.execute(deployerAddress, network.oracleContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    network.uosmoOraclePriceSet = true;
    writeArtifact(network, chain_id);
  }

  msg = {
    price: {
      denom: OSMOSIS_DENOM,
    },
  };
  console.log(
    `${chain_id} :: uosmo oracle price :  ${await client.queryContractSmart(network.oracleContractAddress, msg)}`,
  );

  /*********************************************************************************************************** */
  /*********************************************************************************************************** */
  // STEP EXECUTE SET ORACLE ASSET UATOM
  /*********************************************************************************************************** */
  /*********************************************************************************************************** */

  if (!network.uatomOraclePriceSet) {
    msg = {
      set_price_source: {
        denom: ATOM_DENOM,
        price_source: {
          fixed: { price: '1.5' },
        },
      },
    };

    result = await client.execute(deployerAddress, network.oracleContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    network.uatomOraclePriceSet = true;
    writeArtifact(network, chain_id);
  }

  msg = {
    price: {
      denom: ATOM_DENOM,
    },
  };
  console.log(
    `${chain_id} :: uatom oracle price :  ${await client.queryContractSmart(network.oracleContractAddress, msg)}`,
  );

  if (!network.smokeTest) {
    /*********************************************************************************************************** */
    /*********************************************************************************************************** */
    // STEP EXECUTE DEPOSIT ATOM ASSET
    /*********************************************************************************************************** */
    /*********************************************************************************************************** */

    msg = { deposit: { denom: ATOM_DENOM } };
    coins = [
      {
        denom: ATOM_DENOM,
        amount: '1_000_000',
      },
    ];

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto', undefined, coins);
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    msg = { user_position: { user_address: deployerAddress } };
    console.log(await client.queryContractSmart(network.redBankContractAddress, msg));

    /*********************************************************************************************************** */
    /*********************************************************************************************************** */
    // STEP EXECUTE BORROW OSMO ASSET
    /*********************************************************************************************************** */
    /*********************************************************************************************************** */

    msg = {
      borrow: {
        denom: ATOM_DENOM,
        amount: '300_000',
      },
    };

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    msg = { user_position: { user_address: deployerAddress } };
    console.log(await client.queryContractSmart(network.redBankContractAddress, msg));

    /*********************************************************************************************************** */
    /*********************************************************************************************************** */
    // STEP EXECUTE REPAY ATOM ASSET
    /*********************************************************************************************************** */
    /*********************************************************************************************************** */

    msg = { repay: { denom: ATOM_DENOM } };
    coins = [
      {
        denom: ATOM_DENOM,
        amount: '300_005',
      },
    ];

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto', undefined, coins);
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    msg = { user_position: { user_address: deployerAddress } };
    console.log(await client.queryContractSmart(network.redBankContractAddress, msg));

    /*********************************************************************************************************** */
    /*********************************************************************************************************** */
    // STEP EXECUTE WITHDRAW ATOM ASSET
    /*********************************************************************************************************** */
    /*********************************************************************************************************** */

    msg = {
      withdraw: {
        denom: ATOM_DENOM,
        amount: '1_000_000',
      },
    };

    result = await client.execute(deployerAddress, network.redBankContractAddress, msg, 'auto');
    wasmEvent = result.logs[0].events.find((e) => e.type === 'wasm');
    console.info('The `wasm` event emitted by the contract execution:', wasmEvent);

    msg = { user_position: { user_address: deployerAddress } };
    console.log(await client.queryContractSmart(network.redBankContractAddress, msg));

    network.smokeTest = true;
    writeArtifact(network, chain_id);
  }
}

function requiredEnvironmentVariables(envVars: string[]) {
  const missing = envVars.filter((v) => process.env[v] === undefined);

  if (missing.length > 0) {
    console.error(`Required environment variables are not set: ${missing.join(', ')}`);
    process.exit(1);
  }
}

main().catch((e) => console.log(e));
