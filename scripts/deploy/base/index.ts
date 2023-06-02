import { setupDeployer } from './setupDeployer'
import { DeploymentConfig } from '../../types/config'
import { printGreen, printRed } from '../../utils/chalk'
import {
  atomAsset,
  osmoAsset,
  atomOracle,
  axlUSDCAsset,
  axlUSDCOracle,
  atomAssetTest,
  axlUSDCAssetTest,
  axlUSDCOracleTest,
  osmoOracle,
  marsAssetTest,
  marsOracleTest,
} from '../osmosis/config'

export const taskRunner = async (config: DeploymentConfig) => {
  const deployer = await setupDeployer(config)

  try {
    await deployer.saveStorage()
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('red-bank', 'mars_red_bank.wasm')
    await deployer.upload('address-provider', 'mars_address_provider.wasm')
    await deployer.upload('incentives', 'mars_incentives.wasm')
    await deployer.upload('oracle', `mars_oracle_${config.chainName}.wasm`)
    await deployer.upload('rewards-collector', `mars_rewards_collector_${config.chainName}.wasm`)

    // Instantiate contracts
    deployer.setOwnerAddr()
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle()
    await deployer.instantiateRewards()
    await deployer.saveDeploymentAddrsToFile()

    // setup
    await deployer.updateAddressProvider()
    await deployer.setRoutes()
    if (config.mainnet) {
      await deployer.initializeAsset(osmoAsset)
      await deployer.initializeAsset(atomAsset)
      await deployer.initializeAsset(axlUSDCAsset)
      await deployer.setOracle(osmoOracle)
      await deployer.setOracle(atomOracle)
      await deployer.setOracle(axlUSDCOracle)
    } else {
      await deployer.initializeAsset(osmoAsset)
      await deployer.initializeAsset(atomAssetTest)
      await deployer.initializeAsset(axlUSDCAssetTest)
      await deployer.initializeAsset(marsAssetTest)
      // await deployer.setOracle(atomOracle) NEED POOL SET
      await deployer.setOracle(osmoOracle)
      await deployer.setOracle(axlUSDCOracleTest)
      await deployer.setOracle(marsOracleTest)
    }

    //run tests
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
