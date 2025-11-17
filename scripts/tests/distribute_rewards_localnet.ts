import { osmosisLocalnetConfig } from '../deploy/osmosis/localnet-config'
import { DeploymentConfig } from '../types/config'
import { getWallet, getAddress, setupClient } from '../deploy/base/setup-deployer'
import { Storage } from '../deploy/base/storage'
import { printBlue, printGreen, printRed, printYellow } from '../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { MarsRewardsCollectorBaseClient } from '../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.client'
import { Action } from '../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'
import { MarsCreditManagerClient } from '../types/generated/mars-credit-manager/MarsCreditManager.client'

/**
 * Test script for distributing rewards from Rewards Collector on local Osmosis environment
 *
 * This script performs the following operations:
 * 1. Check current balances in Rewards Collector (Red Bank + Credit Manager)
 * 2. Withdraw all rewards from Red Bank
 * 3. Withdraw all rewards from Credit Manager (Account ID 1)
 * 4. Distribute all collected rewards for each denom
 */

const DEFAULT_LABEL = 'deployer-owner'

interface ScriptOptions {
  label: string
  config: DeploymentConfig
}

const loadOptions = (): ScriptOptions => {
  // Use localnet config
  const baseConfig = osmosisLocalnetConfig

  const mnemonic = process.env.MARS_MNEMONIC ?? baseConfig.deployerMnemonic

  const label = process.env.MARS_LABEL ?? DEFAULT_LABEL

  const config: DeploymentConfig = {
    ...baseConfig,
    deployerMnemonic: mnemonic,
  }

  return { label, config }
}

const getRewardsCollectorBalances = async (
  client: SigningCosmWasmClient,
  rewardsCollectorAddr: string,
  redBankAddr: string,
  creditManagerAddr: string,
  creditAccountId: string,
): Promise<{ denom: string; amount: string }[]> => {
  const balances: { denom: string; amount: string }[] = []

  // Get native wallet balances
  const balance = await client.getBalance(rewardsCollectorAddr, "uusdc")
  balances.push({ denom: balance.denom, amount: balance.amount })

  // Get Red Bank balances
  try {
    const userPosition = await client.queryContractSmart(redBankAddr, {
      user_position: { user: rewardsCollectorAddr },
    })
    if (userPosition.deposits && userPosition.deposits.length > 0) {
      for (const deposit of userPosition.deposits) {
        if (Number(deposit.amount) > 0) {
          balances.push({ denom: deposit.denom, amount: deposit.amount })
        }
      }
    }
  } catch (err) {
    printYellow(`Could not query Red Bank position: ${err}`)
  }

  // Get Credit Manager account balances
  try {
    const cmClient = new MarsCreditManagerClient(client, rewardsCollectorAddr, creditManagerAddr)
    const accountPositions = await cmClient.positions({ accountId: creditAccountId })
    if (accountPositions.deposits && accountPositions.deposits.length > 0) {
      for (const deposit of accountPositions.deposits) {
        if (Number(deposit.amount) > 0) {
          balances.push({ denom: deposit.denom, amount: deposit.amount })
        }
      }
    }
  } catch (err) {
    printYellow(`Could not query Credit Manager account: ${err}`)
  }

  return balances
}

const printBalances = (balances: { denom: string; amount: string }[], title: string) => {
  printYellow(title)
  if (balances.length === 0) {
    printYellow('  No balances found')
  } else {
    for (const balance of balances) {
      printGreen(`  ${balance.denom}: ${balance.amount}`)
    }
  }
}

const main = async () => {
  const { config, label } = loadOptions()
  const wallet = await getWallet(config.deployerMnemonic, config.chain.prefix)
  const client = await setupClient(config, wallet)
  const userAddr = await getAddress(wallet)

  printYellow(`Running Rewards Distribution test with user ${userAddr} on ${config.chain.id}`)

  const storage = await Storage.load(config.chain.id, label)

  // Verify required addresses
  if (!storage.addresses.rewardsCollector) {
    throw new Error('Rewards Collector address not found in storage!')
  }
  if (!storage.addresses.redBank) {
    throw new Error('Red Bank address not found in storage!')
  }
  if (!storage.addresses.creditManager) {
    throw new Error('Credit Manager address not found in storage!')
  }

  const rewardsCollectorAddr = storage.addresses.rewardsCollector
  const rewardsCollectorAccountId = '1'

  printBlue('\n=== Step 1: Check Current Rewards Collector Balances ===')
  const balancesBefore = await getRewardsCollectorBalances(
    client,
    rewardsCollectorAddr,
    storage.addresses.redBank,
    storage.addresses.creditManager,
    rewardsCollectorAccountId,
  )
  printBalances(balancesBefore, 'Balances before withdrawal:')

  // Initialize Rewards Collector client
  const rcClient = new MarsRewardsCollectorBaseClient(client, userAddr, rewardsCollectorAddr)

  // Step 2: Withdraw from Red Bank
  printBlue('\n=== Step 2: Withdraw All Rewards from Red Bank ===')
  const denoms = [config.chain.baseDenom, config.atomDenom, 'uusdc']

  for (const denom of denoms) {
    try {
      printYellow(`Withdrawing ${denom} from Red Bank...`)
      const result = await rcClient.withdrawFromRedBank(
        {
          denom,
          // Not specifying amount will withdraw all available
        },
        'auto',
      )
      printGreen(`✓ Withdrew ${denom} :: TX ${result.transactionHash}`)
    } catch (err) {
      printRed(`Failed to withdraw ${denom} from Red Bank: ${err}`)
      // Continue with other denoms
    }
  }

  // Step 3: Withdraw from Credit Manager
  printBlue('\n=== Step 3: Withdraw All Rewards from Credit Manager ===')

  for (const denom of denoms) {
    try {
      printYellow(`Withdrawing ${denom} from Credit Manager account #${rewardsCollectorAccountId}...`)

      const actions: Action[] = [
        {
          withdraw: {
            denom,
            amount: 'account_balance',
          },
        },
      ]

      const result = await rcClient.withdrawFromCreditManager(
        {
          accountId: rewardsCollectorAccountId,
          actions,
        },
        'auto',
      )
      printGreen(`✓ Withdrew ${denom} :: TX ${result.transactionHash}`)
    } catch (err) {
      printRed(`Failed to withdraw ${denom} from Credit Manager: ${err}`)
      // Continue with other denoms
    }
  }

  // Step 4: Check balances after withdrawal
  printBlue('\n=== Step 4: Check Balances After Withdrawal ===')
  const balancesAfterWithdrawal = await getRewardsCollectorBalances(
    client,
    rewardsCollectorAddr,
    storage.addresses.redBank,
    storage.addresses.creditManager,
    rewardsCollectorAccountId,
  )
  printBalances(balancesAfterWithdrawal, 'Balances after withdrawal:')

  // Step 5: Distribute rewards
  printBlue('\n=== Step 5: Distribute Rewards ===')

  for (const balance of balancesAfterWithdrawal) {
    if (Number(balance.amount) > 0) {
      try {
        printYellow(`Distributing rewards for ${balance.denom} (amount: ${balance.amount})...`)
        const result = await rcClient.distributeRewards(
          {
            denom: balance.denom,
          },
          'auto',
        )
        printGreen(`✓ Distributed ${balance.denom} :: TX ${result.transactionHash}`)
      } catch (err) {
        printRed(`Failed to distribute ${balance.denom}: ${err}`)
        // Continue with other denoms
      }
    }
  }

  // Step 6: Final balance check
  printBlue('\n=== Step 6: Final Balances ===')
  const balancesAfterDistribution = await getRewardsCollectorBalances(
    client,
    rewardsCollectorAddr,
    storage.addresses.redBank,
    storage.addresses.creditManager,
    rewardsCollectorAccountId,
  )
  printBalances(balancesAfterDistribution, 'Balances after distribution:')

  printGreen('\n✅ Rewards distribution flow completed!')

  printBlue('\n=== Summary ===')
  printYellow(`Total denoms processed: ${balancesAfterWithdrawal.length}`)
  printYellow('Operations:')
  printYellow('  - Withdrew rewards from Red Bank')
  printYellow('  - Withdrew rewards from Credit Manager')
  printYellow('  - Distributed rewards to fee collectors and safety fund')
}

void main().catch((err) => {
  printRed(`Test failed: ${String(err)}`)
  console.error(err)
  process.exitCode = 1
})
