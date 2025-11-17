import { osmosisLocalnetConfig } from '../deploy/osmosis/localnet-config'
import { getWallet, getAddress, setupClient } from '../deploy/base/setup-deployer'
import { Storage } from '../deploy/base/storage'
import { printBlue, printGreen, printRed, printYellow } from '../utils/chalk'

/**
 * Script to check and display the Rewards Collector configuration
 *
 * This will show the current config including transfer types for
 * safety fund, fee collector, and revenue share.
 */

const main = async () => {
  const config = osmosisLocalnetConfig
  const wallet = await getWallet(config.deployerMnemonic, config.chain.prefix)
  const client = await setupClient(config, wallet)
  const userAddr = await getAddress(wallet)

  printYellow(`Checking Rewards Collector config on ${config.chain.id}`)
  printYellow(`User: ${userAddr}`)

  const storage = await Storage.load(config.chain.id, 'deployer-owner')

  console.log('storage', storage)
  if (!storage.addresses.rewardsCollector) {
    printRed('Rewards Collector address not found in storage!')
    return
  }

  printGreen(`\nRewards Collector: ${storage.addresses.rewardsCollector}`)

  // Query the config
  printBlue('\n=== Current Rewards Collector Configuration ===')
  try {
    const rcConfig = await client.queryContractSmart(storage.addresses.rewardsCollector, {
      config: {},
    })

    printYellow('\nOwner:')
    printGreen(`  ${rcConfig.owner || 'Not set'}`)

    printYellow('\nAddress Provider:')
    printGreen(`  ${rcConfig.address_provider}`)

    printYellow('\nChannel ID (for IBC):')
    printGreen(`  ${rcConfig.channel_id}`)

    printYellow('\nTimeout (seconds):')
    printGreen(`  ${rcConfig.timeout_seconds}`)

    printYellow('\nSlippage Tolerance:')
    printGreen(`  ${rcConfig.slippage_tolerance}`)

    printYellow('\nSafety Fund Config:')
    printGreen(`  Target Denom: ${rcConfig.safety_fund_config.target_denom}`)
    printGreen(`  Transfer Type: ${rcConfig.safety_fund_config.transfer_type}`)
    printGreen(`  Tax Rate: ${rcConfig.safety_tax_rate}`)

    printYellow('\nFee Collector Config:')
    printGreen(`  Target Denom: ${rcConfig.fee_collector_config.target_denom}`)
    printGreen(`  Transfer Type: ${rcConfig.fee_collector_config.transfer_type}`)

    printYellow('\nRevenue Share Config:')
    printGreen(`  Target Denom: ${rcConfig.revenue_share_config.target_denom}`)
    printGreen(`  Transfer Type: ${rcConfig.revenue_share_config.transfer_type}`)
    printGreen(`  Tax Rate: ${rcConfig.revenue_share_tax_rate}`)

    // Check if we're using IBC when we shouldn't be
    printBlue('\n=== Analysis ===')
    const usingIBC =
      rcConfig.safety_fund_config.transfer_type === 'ibc' ||
      rcConfig.fee_collector_config.transfer_type === 'ibc' ||
      rcConfig.revenue_share_config.transfer_type === 'ibc'

    if (usingIBC) {
      printRed('⚠️  WARNING: Configuration uses IBC transfers!')
      printYellow('For localnet, IBC transfers will fail. You should use "bank" transfer type instead.')
      printYellow('\nTo fix this, you need to update the config using update_config message.')
      printYellow('Run: npx tsx scripts/tests/update_rewards_collector_config.ts')
    } else {
      printGreen('✓ Configuration uses bank transfers (correct for localnet)')
    }
  } catch (err) {
    printRed(`Failed to query config: ${err}`)
    throw err
  }
}

void main().catch((err) => {
  printRed(`Check failed: ${String(err)}`)
  console.error(err)
  process.exitCode = 1
})
