import { osmosisLocalnetConfig } from '../deploy/osmosis/localnet-config'
import { DeploymentConfig } from '../types/config'
import { getWallet, getAddress, setupClient } from '../deploy/base/setup-deployer'
import { Storage } from '../deploy/base/storage'
import { printBlue, printGreen, printRed, printYellow } from '../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { readFile } from 'fs/promises'
import path from 'path'
import { MarsCreditManagerClient } from '../types/generated/mars-credit-manager/MarsCreditManager.client'
import { Action } from '../types/generated/mars-credit-manager/MarsCreditManager.types'
import { MarsAccountNftQueryClient } from '../types/generated/mars-account-nft/MarsAccountNft.client'

/**
 * Test script for Rewards Collector on local Osmosis environment
 *
 * This script performs the following operations using Credit Manager:
 * 0. Deploy mock Astroport incentives contract and set it in address provider
 * 1. Check balances (ATOM, USDC, OSMO)
 * 2. Create a credit account
 * 3. Deposit ATOM to credit account
 * 3.5. Deposit USDC to Red Bank (to provide liquidity for borrowing)
 * 4. Borrow USDC from Red Bank via credit account
 * 5. Swap USDC -> ATOM via credit account
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

const ensureBalance = async (
  client: SigningCosmWasmClient,
  address: string,
  denom: string,
  minAmount: string,
) => {
  const balance = await client.getBalance(address, denom)
  printBlue(`Balance of ${denom}: ${balance.amount}`)
  if (Number(balance.amount) < Number(minAmount)) {
    throw new Error(
      `Insufficient ${denom} balance. Required: ${minAmount}, Available: ${balance.amount}`,
    )
  }
}

const deployMockAstroportIncentives = async (
  client: SigningCosmWasmClient,
  deployerAddr: string,
): Promise<string> => {
  printBlue('Deploying mock Astroport incentives contract...')

  // Path to the mock astroport incentives wasm file
  const wasmPath = path.resolve(__dirname, '../../artifacts/mars_mock_astroport_incentives.wasm')

  try {
    const wasm = await readFile(wasmPath)
    printYellow(`Uploading mock contract from ${wasmPath}`)

    const { codeId } = await client.upload(deployerAddr, new Uint8Array(wasm), 'auto')
    printGreen(`✓ Uploaded mock Astroport incentives :: code id ${codeId}`)

    // Instantiate the mock contract
    const instantiateMsg = {
      owner: deployerAddr,
    }

    const { contractAddress } = await client.instantiate(
      deployerAddr,
      codeId,
      instantiateMsg,
      'Mock Astroport Incentives',
      'auto',
    )

    printGreen(`✓ Instantiated mock Astroport incentives at ${contractAddress}`)
    return contractAddress
  } catch (err) {
    printRed(`Failed to deploy mock Astroport incentives: ${err}`)
    throw err
  }
}

const setAstroportIncentivesAddress = async (
  client: SigningCosmWasmClient,
  deployerAddr: string,
  addressProviderAddr: string,
  astroportIncentivesAddr: string,
): Promise<void> => {
  printBlue('Setting astroport_incentives address in address provider...')

  const msg = {
    set_address: {
      address_type: 'astroport_incentives',
      address: astroportIncentivesAddr,
    },
  }

  try {
    const result = await client.execute(deployerAddr, addressProviderAddr, msg, 'auto')
    printGreen(`✓ Set astroport_incentives address :: TX ${result.transactionHash}`)
  } catch (err) {
    printRed(`Failed to set address: ${err}`)
    throw err
  }
}

const checkAstroportIncentivesAddress = async (
  client: SigningCosmWasmClient,
  addressProviderAddr: string,
): Promise<string | null> => {
  try {
    const result = await client.queryContractSmart(addressProviderAddr, {
      address: 'astroport_incentives',
    })
    return result as string
  } catch (err) {
    return null
  }
}

const swapViaAccount = async (
  client: SigningCosmWasmClient,
  userAddr: string,
  creditManagerAddr: string,
  accountId: string,
  amountIn: string,
  denomIn: string,
  denomOut: string,
  poolId: number,
): Promise<string> => {
  printBlue(`Swapping ${amountIn} ${denomIn} for ${denomOut} via pool ${poolId}`)

  const actions: Action[] = [
    {
      swap_exact_in: {
        coin_in: {
          amount: { exact: amountIn },
          denom: denomIn,
        },
        denom_out: denomOut,
        min_receive: '100',
        route: {
          osmo: {
            swaps: [
              {
                pool_id: poolId,
                to: denomOut,
              },
            ],
          },
        },
      },
    },
  ]

  const cmClient = new MarsCreditManagerClient(client, userAddr, creditManagerAddr)
  const result = await cmClient.updateCreditAccount(
    {
      accountId,
      actions,
    },
    'auto',
  )

  printYellow(`TX: ${result.transactionHash}`)
  printGreen(`✓ Swap successful`)
  return result.transactionHash
}

const main = async () => {
  const { config, label } = loadOptions()
  const wallet = await getWallet(config.deployerMnemonic, config.chain.prefix)
  const client = await setupClient(config, wallet)
  const userAddr = await getAddress(wallet)

  printYellow(`Running Rewards Collector test with user ${userAddr} on ${config.chain.id}`)

  const storage = await Storage.load(config.chain.id, label)

  // Step 0: Check if astroport_incentives address is set, if not deploy and set it
  if (!storage.addresses.addressProvider) {
    throw new Error('Address provider not found in storage!')
  }

  printBlue('\n=== Step 0: Setup Astroport Incentives Address ===')
  const existingAddr = await checkAstroportIncentivesAddress(client, storage.addresses.addressProvider)

  if (existingAddr) {
    printGreen(`✓ astroport_incentives already set to: ${existingAddr}`)
  } else {
    printYellow('astroport_incentives not set, deploying mock contract...')
    const mockAddr = await deployMockAstroportIncentives(client, userAddr)
    await setAstroportIncentivesAddress(client, userAddr, storage.addresses.addressProvider, mockAddr)
    printGreen(`✓ astroport_incentives now set to: ${mockAddr}`)
  }

  // Test parameters - customize these for your needs
  const atomDenom = config.atomDenom // uatom
  const usdcDenom = 'uusdc' // Local USDC
  const depositAmount = '10000000' // 10 ATOM (6 decimals)
  const borrowAmount = '5000000'   // 5 USDC (6 decimals)
  const swapAmount = '3000000'     // 3 USDC to swap
  const atomUsdcPoolId = 3 // Pool ID for ATOM/USDC from localnet-config

  printBlue('\n=== Step 1: Check balances ===')
  await ensureBalance(client, userAddr, atomDenom, depositAmount)
  await ensureBalance(client, userAddr, usdcDenom, '20000000') // For Red Bank deposit
  await ensureBalance(client, userAddr, config.chain.baseDenom, '100000') // For gas

  printBlue('\n=== Step 2: Create Credit Account ===')
  const cmClient = new MarsCreditManagerClient(client, userAddr, storage.addresses.creditManager!)

  // Get tokens before creating account
  const nftClient = new MarsAccountNftQueryClient(client, storage.addresses.accountNft!)
  const tokensBefore = await nftClient.tokens({ owner: userAddr })

  // Create account using execute message directly
  const createAccountResult = await client.execute(
    userAddr,
    storage.addresses.creditManager!,
    { create_credit_account: 'default' },
    'auto',
  )
  printYellow(`TX: ${createAccountResult.transactionHash}`)

  // Get the new account ID
  const tokensAfter = await nftClient.tokens({ owner: userAddr })
  const newTokens = tokensAfter.tokens.filter((t) => !tokensBefore.tokens.includes(t))
  const accountId = newTokens[0]
  printGreen(`✓ Created credit account: #${accountId}`)

  printBlue('\n=== Step 3: Deposit ATOM to Credit Account ===')
  const depositResult = await cmClient.updateCreditAccount(
    {
      accountId,
      actions: [{ deposit: { amount: depositAmount, denom: atomDenom } }],
    },
    'auto',
    undefined,
    [{ amount: depositAmount, denom: atomDenom }],
  )
  printYellow(`TX: ${depositResult.transactionHash}`)
  printGreen(`✓ Deposited ${depositAmount} ${atomDenom} to credit account`)

  // Step 3.5: Deposit USDC to Red Bank so we can borrow it
  printBlue('\n=== Step 3.5: Deposit USDC to Red Bank (for borrowing) ===')
  if (!storage.addresses.redBank) {
    throw new Error('Red Bank address not found in storage!')
  }

  const usdcDepositAmount = '20000000' // 20 USDC to ensure we have enough liquidity
  printYellow(`Depositing ${usdcDepositAmount} ${usdcDenom} to Red Bank for liquidity...`)

  const redBankDepositResult = await client.execute(
    userAddr,
    storage.addresses.redBank,
    { deposit: {} },
    'auto',
    undefined,
    [{ denom: usdcDenom, amount: usdcDepositAmount }],
  )
  printYellow(`TX: ${redBankDepositResult.transactionHash}`)
  printGreen(`✓ Deposited ${usdcDepositAmount} ${usdcDenom} to Red Bank`)

  printBlue('\n=== Step 4: Borrow USDC from Red Bank ===')
  const borrowResult = await cmClient.updateCreditAccount(
    {
      accountId,
      actions: [{ borrow: { amount: borrowAmount, denom: usdcDenom } }],
    },
    'auto',
  )
  printYellow(`TX: ${borrowResult.transactionHash}`)
  printGreen(`✓ Borrowed ${borrowAmount} ${usdcDenom} from Red Bank`)

  printBlue('\n=== Step 5: Swap USDC -> ATOM ===')
  const swapTxHash = await swapViaAccount(
    client,
    userAddr,
    storage.addresses.creditManager!,
    accountId,
    swapAmount,
    usdcDenom,
    atomDenom,
    atomUsdcPoolId,
  )

  printGreen('\n✅ Credit Manager test flow completed successfully!')
  printYellow('\nThe credit account now has:')
  printYellow('- Initial ATOM deposit')
  printYellow('- Borrowed USDC (partially swapped back to ATOM)')
  printYellow('- USDC debt')

  printBlue('\n=== Transaction Summary for Audit ===')
  printYellow(`Create Credit Account: ${createAccountResult.transactionHash}`)
  printYellow(`Deposit ATOM: ${depositResult.transactionHash}`)
  printYellow(`Deposit USDC to Red Bank: ${redBankDepositResult.transactionHash}`)
  printYellow(`Borrow USDC: ${borrowResult.transactionHash}`)
  printYellow(`Swap USDC->ATOM: ${swapTxHash}`)

  // Check Rewards Collector balances
  printBlue('\n=== Rewards Collector Balances ===')
  const rewardsCollectorAddr = 'osmo1eyfccmjm6732k7wp4p6gdjwhxjwsvje44j0hfx8nkgrm8fs7vqfsn92ayh'
  const rewardsCollectorAccountId = '1'

  // Check Credit Manager account balances (Account ID 1)
  try {
    const accountPositions = await cmClient.positions({ accountId: rewardsCollectorAccountId })
    printYellow(`\nRewards Collector Credit Account #${rewardsCollectorAccountId} positions:`)
    if (accountPositions.deposits && accountPositions.deposits.length > 0) {
      accountPositions.deposits.forEach((deposit: any) => {
        printGreen(`  ${deposit.denom}: ${deposit.amount}`)
      })
    } else {
      printYellow('  No deposits found')
    }
    if (accountPositions.debts && accountPositions.debts.length > 0) {
      printYellow('  Debts:')
      accountPositions.debts.forEach((debt: any) => {
        printRed(`    ${debt.denom}: ${debt.amount}`)
      })
    }
  } catch (err) {
    printRed(`Failed to query Credit Manager account: ${err}`)
  }

  // Check Red Bank balances
  try {
    const userPosition = await client.queryContractSmart(storage.addresses.redBank!, {
      user_position: { user: rewardsCollectorAddr },
    })
    printYellow(`\nRewards Collector Red Bank balances (${rewardsCollectorAddr}):`)
    if (userPosition.deposits && userPosition.deposits.length > 0) {
      userPosition.deposits.forEach((deposit: any) => {
        printGreen(`  ${deposit.denom}: ${deposit.amount}`)
      })
    } else {
      printYellow('  No deposits found')
    }
    if (userPosition.debts && userPosition.debts.length > 0) {
      printYellow('  Debts:')
      userPosition.debts.forEach((debt: any) => {
        printRed(`    ${debt.denom}: ${debt.amount}`)
      })
    }
  } catch (err) {
    printRed(`Failed to query Red Bank position: ${err}`)
  }
}

void main().catch((err) => {
  printRed(`Test failed: ${String(err)}`)
  console.error(err)
  process.exitCode = 1
})
