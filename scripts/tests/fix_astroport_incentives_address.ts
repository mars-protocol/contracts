import { osmosisLocalnetConfig } from '../deploy/osmosis/localnet-config'
import { getWallet, getAddress, setupClient } from '../deploy/base/setup-deployer'
import { Storage } from '../deploy/base/storage'
import { printBlue, printGreen, printRed, printYellow } from '../utils/chalk'

/**
 * Fix script to set astroport_incentives address in the address provider
 *
 * Since we're on Osmosis (not Neutron/Astroport), we'll point it to the
 * Osmosis incentives contract as a workaround.
 */

const main = async () => {
  const config = osmosisLocalnetConfig
  const wallet = await getWallet(config.deployerMnemonic, config.chain.prefix)
  const client = await setupClient(config, wallet)
  const userAddr = await getAddress(wallet)

  printYellow(`Fixing astroport_incentives address on ${config.chain.id}`)
  printYellow(`User: ${userAddr}`)

  const storage = await Storage.load(config.chain.id, 'deployer-owner')

  if (!storage.addresses.addressProvider) {
    printRed('Address provider not found in storage!')
    return
  }

  if (!storage.addresses.incentives) {
    printRed('Incentives contract not found in storage!')
    return
  }

  printGreen(`Address Provider: ${storage.addresses.addressProvider}`)
  printGreen(`Incentives Contract: ${storage.addresses.incentives}`)

  // Check current owner
  printBlue('\nChecking address provider owner...')
  const ownerInfo = await client.queryContractSmart(storage.addresses.addressProvider, {
    config: {},
  })
  printYellow(`Owner: ${ownerInfo.owner}`)

  if (ownerInfo.owner !== userAddr) {
    printRed(`Error: You (${userAddr}) are not the owner (${ownerInfo.owner})`)
    printYellow('You need to run this script with the owner wallet')
    return
  }

  // Set astroport_incentives to point to the osmosis incentives contract
  printBlue('\nSetting astroport_incentives address...')
  const msg = {
    set_address: {
      address_type: 'astroport_incentives',
      address: storage.addresses.incentives, // Point to Osmosis incentives
    },
  }

  try {
    const result = await client.execute(
      userAddr,
      storage.addresses.addressProvider,
      msg,
      'auto',
    )
    printGreen(`✓ Transaction successful!`)
    printYellow(`TX Hash: ${result.transactionHash}`)
  } catch (err) {
    printRed(`Failed to update address: ${err}`)
    throw err
  }

  // Verify the update
  printBlue('\nVerifying update...')
  try {
    const result = await client.queryContractSmart(storage.addresses.addressProvider, {
      address: 'astroport_incentives',
    })
    printGreen(`✓ astroport_incentives is now set to: ${result}`)
  } catch (err) {
    printRed(`Failed to verify: ${err}`)
  }

  printGreen('\n✅ Fix completed successfully!')
  printYellow('\nYou can now run the test script:')
  printYellow('  npx ts-node tests/test_rewards_collector_localnet.ts')
}

void main().catch((err) => {
  printRed(`Fix failed: ${String(err)}`)
  console.error(err)
  process.exitCode = 1
})
