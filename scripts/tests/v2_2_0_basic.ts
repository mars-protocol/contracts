import path from 'path'
import { readFile } from 'fs/promises'
import { osmosisTestnetConfig } from '../deploy/osmosis/testnet-config'
import { DeploymentConfig } from '../types/config'
import { getWallet, getAddress, setupClient } from '../deploy/base/setup-deployer'
import { Storage } from '../deploy/base/storage'
import { printBlue, printGreen, printRed, printYellow } from '../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { MarsCreditManagerQueryClient } from '../types/generated/mars-credit-manager/MarsCreditManager.client'
import { Rover } from '../deploy/base/test-actions-credit-manager'
import { Deployer as RedBankHarness } from '../deploy/base/test-actions-red-bank'

const TARGET_VERSION = '2.2.0'
const ARTIFACT_FILE = 'mars_credit_manager.wasm'
const DEFAULT_LABEL = 'deployer-owner'

type SupportedEnv = 'osmosis-testnet'

const supportedConfigs: Record<SupportedEnv, DeploymentConfig> = {
  'osmosis-testnet': osmosisTestnetConfig,
}

interface ScriptOptions {
  label: string
  config: DeploymentConfig
}

const expectDefined = <T>(value: T | undefined | null, message: string): T => {
  if (value === undefined || value === null) {
    throw new Error(message)
  }
  return value
}

const resolveFromRepoRoot = (...segments: string[]) =>
  path.resolve(__dirname, '../../../', ...segments)

const loadOptions = (): ScriptOptions => {
  const env = (process.env.MARS_ENV ?? 'osmosis-testnet') as SupportedEnv
  const baseConfig = supportedConfigs[env]
  if (!baseConfig) {
    throw new Error(`Unsupported MARS_ENV value: ${env}`)
  }

  const mnemonic = process.env.MARS_MNEMONIC ?? baseConfig.deployerMnemonic
  if (!mnemonic || mnemonic.includes('TO BE INSERTED')) {
    throw new Error('Set MARS_MNEMONIC with the deployer mnemonic for this environment.')
  }

  const label = process.env.MARS_LABEL ?? DEFAULT_LABEL

  return {
    label,
    config: { ...baseConfig, deployerMnemonic: mnemonic },
  }
}

const uploadCreditManager = async (
  client: SigningCosmWasmClient,
  deployer: string,
  storage: Storage,
) => {
  const wasmPath = resolveFromRepoRoot('artifacts', ARTIFACT_FILE)
  const wasm = await readFile(wasmPath)
  printBlue(`Uploading ${ARTIFACT_FILE} from ${wasmPath}`)
  const { codeId, checksum } = await client.upload(deployer, wasm, 'auto')
  storage.codeIds.creditManager = codeId
  printGreen(`Uploaded credit manager wasm :: code id ${codeId} :: checksum ${checksum}`)
  return codeId
}

const migrateCreditManager = async (
  client: SigningCosmWasmClient,
  deployer: string,
  storage: Storage,
  expectedVersion: string,
) => {
  const creditManagerAddr = expectDefined(
    storage.addresses.creditManager,
    'credit manager address missing from storage; deploy or record it first.',
  )

  const info: { contract: string; version: string } = await client.queryContractSmart(
    creditManagerAddr,
    { contract_version: {} },
  )

  if (info.version === expectedVersion) {
    printYellow(`Credit manager already at target version ${expectedVersion}, skipping migrate.`)
    return
  }

  printBlue(
    `Migrating credit manager at ${creditManagerAddr} from version ${info.version} to ${expectedVersion}`,
  )
  const codeId = await uploadCreditManager(client, deployer, storage)
  await client.migrate(deployer, creditManagerAddr, codeId, {}, "auto")

  const after: { contract: string; version: string } = await client.queryContractSmart(
    creditManagerAddr,
    { contract_version: {} },
  )

  if (after.version !== expectedVersion) {
    throw new Error(`Migration failed: expected ${expectedVersion}, found ${after.version}`)
  }
  printGreen(`Migration complete :: contract ${after.contract} :: version ${after.version}`)
}

const ensureSwapFeeInitialized = async (
  client: SigningCosmWasmClient,
  storage: Storage,
) => {
  const creditManagerAddr = expectDefined(
    storage.addresses.creditManager,
    'credit manager address missing from storage.',
  )
  const queryClient = new MarsCreditManagerQueryClient(client, creditManagerAddr)
  const fee = await queryClient.swapFeeRate()
  printGreen(`Swap fee rate query succeeded :: rate ${fee}`)
}

const runCreditAccountFlow = async (
  client: SigningCosmWasmClient,
  deployerAddr: string,
  storage: Storage,
  config: DeploymentConfig,
) => {
  if (!config.testActions) {
    throw new Error('testActions not configured for deployment; cannot run credit account flow.')
  }

  const rover = new Rover(deployerAddr, storage, config, client, config.testActions)
  await rover.createCreditAccount()
  await rover.deposit()
  await rover.swap()
  await rover.withdraw()
  await rover.refundAllBalances()
  printGreen('Credit account flow complete')
}

const distributeRewards = async (
  client: SigningCosmWasmClient,
  deployerAddr: string,
  storage: Storage,
  config: DeploymentConfig,
) => {
  const rewardsAddr = storage.addresses.rewardsCollector
  if (!rewardsAddr) {
    printYellow('Rewards collector address missing; skipping reward distribution checks.')
    return
  }
  const contractAddr = rewardsAddr

  const denoms = new Set<string>([
    config.rewardsCollector.safetyFundConfig.target_denom,
    config.rewardsCollector.revenueShareConfig.target_denom,
    config.rewardsCollector.feeCollectorConfig.target_denom,
  ])

  for (const denom of denoms) {
    printBlue(`Triggering distribute_rewards for denom ${denom}`)
    try {
      await client.execute(
        deployerAddr,
        contractAddr,
        { distribute_rewards: { denom } },
        'auto',
      )
      printGreen(`distribute_rewards executed for ${denom}`)
    } catch (err) {
      printRed(`distribute_rewards failed for ${denom}: ${String(err)}`)
      throw err
    }
  }
}

const runRewardsSwap = async (
  client: SigningCosmWasmClient,
  deployerAddr: string,
  storage: Storage,
  config: DeploymentConfig,
) => {
  const harness = new RedBankHarness(config, client, deployerAddr, storage)
  await harness.executeRewardsSwap()
}

const ensureGasBalance = async (
  client: SigningCosmWasmClient,
  address: string,
  denom: string,
  minAmount: number,
) => {
  const balance = await client.getBalance(address, denom)
  if (Number(balance.amount) < minAmount) {
    throw new Error(`Insufficient ${denom} balance (${balance.amount}) to run tests.`)
  }
}

const main = async () => {
  const { config, label } = loadOptions()
  const wallet = await getWallet(config.deployerMnemonic, config.chain.prefix)
  const client = await setupClient(config, wallet)
  const deployerAddr = await getAddress(wallet)

  printYellow(`Running tests with deployer ${deployerAddr} on ${config.chain.id}`)

  const storage = await Storage.load(config.chain.id, label)

  await ensureGasBalance(client, deployerAddr, config.chain.baseDenom, 100_000)
  await migrateCreditManager(client, deployerAddr, storage, TARGET_VERSION)
  await ensureSwapFeeInitialized(client, storage)
  await runCreditAccountFlow(client, deployerAddr, storage, config)
  await distributeRewards(client, deployerAddr, storage, config)
  await runRewardsSwap(client, deployerAddr, storage, config)

  await storage.save()
  printGreen('v2.2.0 basic functionality script completed successfully')
}

void main().catch((err) => {
  printRed(`v2.2.0 basic functionality script failed: ${String(err)}`)
  process.exitCode = 1
})
