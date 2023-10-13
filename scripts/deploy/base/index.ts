import { setupDeployer } from './setup-deployer'
import { printGreen, printRed, printYellow } from '../../utils/chalk'
import { DeploymentConfig } from '../../types/config'
import { wasmFile } from '../../utils/environment'

export interface TaskRunnerProps {
  config: DeploymentConfig
  label: string
}

export const taskRunner = async ({ config, label }: TaskRunnerProps) => {
  const deployer = await setupDeployer(config, label)

  try {
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('redBank', 'mars_red_bank.wasm')
    await deployer.upload('addressProvider', 'mars_address_provider.wasm')
    await deployer.upload('incentives', 'mars_incentives.wasm')
    await deployer.upload('oracle', `mars_oracle_${config.oracle.name}.wasm`)
    await deployer.upload(
      'rewardsCollector',
      `mars_rewards_collector_${config.rewardsCollector.name}.wasm`,
    )
    await deployer.upload('swapper', `mars_swapper_${config.swapper.name}.wasm`)
    await deployer.upload('params', `mars_params.wasm`)
    await deployer.upload('accountNft', wasmFile('mars_account_nft'))
    await deployer.upload('mockVault', wasmFile('mars_mock_vault'))
    await deployer.upload('zapper', wasmFile(config.zapperContractName))
    await deployer.upload('creditManager', wasmFile('mars_credit_manager'))
    await deployer.upload('health', wasmFile('mars_rover_health'))

    // Instantiate contracts
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle(config.oracle.customInitParams)
    await deployer.instantiateRewards()
    await deployer.instantiateSwapper()
    await deployer.instantiateParams()
    await deployer.instantiateMockVault()
    await deployer.instantiateZapper()
    await deployer.instantiateHealthContract()
    await deployer.instantiateCreditManager()
    await deployer.instantiateNftContract()
    await deployer.setConfigOnHealthContract()
    await deployer.transferNftContractOwnership()
    await deployer.setConfigOnCreditManagerContract()
    await deployer.saveDeploymentAddrsToFile(label)

    await deployer.updateAddressProvider()

    // setup
    for (const asset of config.assets) {
      await deployer.updateAssetParams(asset)
      await deployer.initializeMarket(asset)
    }
    for (const vault of config.vaults) {
      await deployer.updateVaultConfig(vault)
    }
    for (const oracleConfig of config.oracleConfigs) {
      await deployer.setOracle(oracleConfig)
    }
    await deployer.setRoutes()

    await deployer.grantCreditLines()

    // Test basic user flows
    if (config.runTests && config.testActions) {
      await deployer.executeDeposit()
      await deployer.executeBorrow()
      await deployer.executeRepay()
      await deployer.executeWithdraw()
      // await deployer.executeRewardsSwap()

      const rover = await deployer.newUserRoverClient(config.testActions)
      await rover.createCreditAccount()
      await rover.deposit()
      await rover.lend()
      await rover.borrow()
      await rover.swap()
      await rover.repay()
      await rover.reclaim()
      await rover.withdraw()

      const vaultConfig = config.vaults[0].vault
      const info = await rover.getVaultInfo(vaultConfig)
      await rover.zap(info.tokens.base_token)
      await rover.vaultDeposit(vaultConfig, info)
      if (info.lockup) {
        await rover.vaultRequestUnlock(vaultConfig, info)
      } else {
        await rover.vaultWithdraw(vaultConfig, info)
        await rover.unzap(info.tokens.base_token)
      }
      await rover.refundAllBalances()
    }

    // If multisig is set, transfer ownership from deployer to multisig
    if (config.multisigAddr) {
      await deployer.updateIncentivesContractOwner()
      await deployer.updateRedBankContractOwner()
      await deployer.updateOracleContractOwner()
      await deployer.updateRewardsContractOwner()
      await deployer.updateSwapperContractOwner()
      await deployer.updateParamsContractOwner()
      await deployer.updateAddressProviderContractOwner()
      await deployer.updateCreditManagerOwner()
      await deployer.updateHealthOwner()
      printGreen('It is confirmed that all contracts have transferred ownership to the Multisig')
    } else {
      printGreen('Owner remains the deployer address.')
    }

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
