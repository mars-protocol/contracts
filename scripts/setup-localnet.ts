import { readFileSync } from 'fs'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice, coin } from '@cosmjs/stargate'
import * as yaml from 'js-yaml'
import { getSigningOsmosisClient } from 'osmojs'
import { osmosis } from 'osmojs'
import { LocalnetConfig, PoolConfig, SeedAddress } from './types/localnet-config'
import { printGreen, printYellow, printRed, printBlue } from './utils/chalk'

const CONFIG_FILE = process.argv[2] || './localnet-config.yaml'
const RPC_ENDPOINT = process.env.RPC_ENDPOINT || 'http://localhost:26657'

interface PoolCreationResult {
  poolId: string
  config: PoolConfig
}

async function loadConfig(): Promise<LocalnetConfig> {
  try {
    const fileContents = readFileSync(CONFIG_FILE, 'utf8')
    const config = yaml.load(fileContents) as LocalnetConfig
    return config
  } catch (error) {
    printRed(`Error loading config file: ${error}`)
    throw error
  }
}

async function setupClient(config: LocalnetConfig) {
  printYellow('Setting up client connection...')

  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(config.genesis_account.mnemonic, {
    prefix: 'osmo',
  })

  const client = await getSigningOsmosisClient({
    rpcEndpoint: RPC_ENDPOINT,
    signer: wallet,
  })

  // Set gas price after client creation
  ;(client as any).gasPrice = GasPrice.fromString(`0.025${config.chain.denom}`)

  const accounts = await wallet.getAccounts()
  const address = accounts[0].address

  printGreen(`Connected to ${RPC_ENDPOINT}`)
  printGreen(`Using account: ${address}`)

  // Check balance
  const balance = await client.getBalance(address, config.chain.denom)
  printBlue(`Balance: ${balance.amount} ${balance.denom}`)

  return { client, address, wallet }
}

async function createPools(
  client: any,
  address: string,
  pools: PoolConfig[]
): Promise<PoolCreationResult[]> {
  printYellow('\nCreating pools...')

  const results: PoolCreationResult[] = []

  for (const pool of pools) {
    try {
      printBlue(`\nCreating ${pool.name}...`)
      printBlue(`  ${pool.token1.amount} ${pool.token1.denom}`)
      printBlue(`  ${pool.token2.amount} ${pool.token2.denom}`)
      printBlue(`  Swap fee: ${pool.swap_fee}, Exit fee: ${pool.exit_fee}`)

      // Create pool message using osmojs
      // Use the poolmodels module for creating balancer pools
      const { createBalancerPool } = osmosis.gamm.poolmodels.balancer.v1beta1.MessageComposer
        .withTypeUrl
      const msg = createBalancerPool({
        sender: address,
        poolParams: {
          swapFee: pool.swap_fee,
          exitFee: pool.exit_fee,
          smoothWeightChangeParams: undefined,
        },
        poolAssets: [
          {
            token: coin(pool.token1.amount, pool.token1.denom),
            weight: '1',
          },
          {
            token: coin(pool.token2.amount, pool.token2.denom),
            weight: '1',
          },
        ],
        futurePoolGovernor: '',
      })

      // Execute transaction
      const result = await client.signAndBroadcast(address, [msg], 'auto', `Create ${pool.name}`)

      if (result.code !== 0) {
        printRed(`Failed to create pool: ${result.rawLog}`)
        throw new Error(`Pool creation failed: ${result.rawLog}`)
      }

      // Extract pool ID from events
      const poolIdEvent = result.events.find((e: any) => e.type === 'pool_created')
      const poolIdAttr = poolIdEvent?.attributes.find((a: any) => a.key === 'pool_id')
      const poolId = poolIdAttr?.value || 'unknown'

      printGreen(`✓ Pool created with ID: ${poolId}`)

      results.push({
        poolId,
        config: pool,
      })
    } catch (error) {
      printRed(`Error creating pool ${pool.name}: ${error}`)
      throw error
    }
  }

  return results
}

async function seedAddresses(
  client: any,
  fromAddress: string,
  seedAddresses: SeedAddress[]
): Promise<void> {
  printYellow('\nSeeding addresses...')

  for (const seedAddr of seedAddresses) {
    try {
      printBlue(`\nSeeding ${seedAddr.name || seedAddr.address}...`)

      // Prepare coins to send and sort alphabetically by denom (required by Cosmos SDK)
      const coins = seedAddr.balances
        .map((b) => coin(b.amount, b.denom))
        .sort((a, b) => a.denom.localeCompare(b.denom))

      // Send tokens
      const result = await client.sendTokens(
        fromAddress,
        seedAddr.address,
        coins,
        'auto',
        `Seed ${seedAddr.name || seedAddr.address}`
      )

      if (result.code !== 0) {
        printRed(`Failed to seed address: ${result.rawLog}`)
        throw new Error(`Seeding failed: ${result.rawLog}`)
      }

      printGreen(`✓ Seeded ${seedAddr.address}`)
      for (const balance of seedAddr.balances) {
        printBlue(`  ${balance.amount} ${balance.denom}`)
      }
    } catch (error) {
      printRed(`Error seeding address ${seedAddr.address}: ${error}`)
      throw error
    }
  }
}

async function main() {
  try {
    printGreen('========================================')
    printGreen('Osmosis Localnet Setup Script')
    printGreen('========================================\n')

    printBlue(`Loading configuration from ${CONFIG_FILE}...`)
    const config = await loadConfig()

    printBlue(`Chain ID: ${config.chain.chain_id}`)
    printBlue(`Assets: ${config.assets.map((a) => a.denom).join(', ')}`)
    printBlue(`Pools to create: ${config.pools.length}`)
    printBlue(`Addresses to seed: ${config.seed_addresses.length}`)

    const { client, address } = await setupClient(config)

    // Wait a bit for the chain to be ready
    printYellow('\nWaiting for chain to be ready...')
    await new Promise((resolve) => setTimeout(resolve, 2000))

    // Create pools
    const poolResults = await createPools(client, address, config.pools)

    // Seed addresses
    await seedAddresses(client, address, config.seed_addresses)

    printGreen('\n========================================')
    printGreen('Setup Complete!')
    printGreen('========================================\n')

    printBlue('Created Pools:')
    poolResults.forEach((result) => {
      printGreen(`  Pool ID ${result.poolId}: ${result.config.name}`)
      printBlue(`    Denom: gamm/pool/${result.poolId}`)
    })

    printBlue('\nSeeded Addresses:')
    config.seed_addresses.forEach((addr) => {
      printGreen(`  ${addr.name || 'Address'}: ${addr.address}`)
    })

    printYellow('\nYour localnet is ready for testing!')
  } catch (error) {
    printRed(`\nSetup failed: ${error}`)
    process.exit(1)
  }
}

main()
