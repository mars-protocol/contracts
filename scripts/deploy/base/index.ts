import { setupDeployer } from './setupDeployer'
import { DeploymentConfig } from '../../types/config'
import { printGreen, printRed } from '../../utils/chalk'

export const taskRunner = async (config: DeploymentConfig) => {
  const deployer = await setupDeployer(config)

  try {
    await deployer.saveStorage()
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('red-bank', 'mars_red_bank.wasm')
    await deployer.upload('address-provider', 'mars_address_provider.wasm')
    await deployer.upload('incentives', 'mars_incentives.wasm')
    await deployer.upload('oracle', `mars_oracle_${config.oracleName}.wasm`)
    await deployer.upload(
      'rewards-collector',
      `mars_rewards_collector_${config.rewardsCollectorName}.wasm`,
    )
    await deployer.upload('swapper', `mars_swapper_${config.swapperDexName}.wasm`)

    // Instantiate contracts
    deployer.setOwnerAddr()
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle(config.oracleCustomInitParams)
    await deployer.instantiateRewards()
    await deployer.instantiateSwapper()
    await deployer.instantiateParams()
    await deployer.saveDeploymentAddrsToFile()

    // setup
    await deployer.updateAddressProvider()
    for (const asset of config.assets) {
      await deployer.updateAssetParams(asset)
    }
    for (const vault of config.vaults) {
      await deployer.updateVaultConfig(vault)
    }
    await deployer.setRoutes()
    for (const oracleConfig of config.oracleConfigs) {
      await deployer.setOracle(oracleConfig)
    }

    // run tests
    if (config.runTests) {
      await deployer.executeDeposit()
      await deployer.executeBorrow()
      await deployer.executeRepay()
      await deployer.executeWithdraw()
      await deployer.executeRewardsSwap()
    }

    if (config.multisigAddr) {
      await deployer.updateIncentivesContractOwner()
      await deployer.updateRedBankContractOwner()
      await deployer.updateOracleContractOwner()
      await deployer.updateRewardsContractOwner()
      await deployer.updateSwapperContractOwner()
      await deployer.updateAddressProviderContractOwner()
      printGreen('It is confirmed that all contracts have transferred ownership to the Multisig')
    } else {
      printGreen('Owner remains the deployer address.')
    }
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
