import { setupDeployer } from './setupDeployer'
import { DeploymentConfig } from '../../types/config'
import { printGreen, printYellow, printRed } from '../../utils/chalk'
import { atomAsset, osmoAsset, osmoOracle, atomOracle } from '../osmosis/config'

export const taskRunner = async (config: DeploymentConfig) => {
  const deployer = await setupDeployer(config)

  try {
    await deployer.saveStorage()
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('redBank', 'mars_red_bank.wasm')
    await deployer.upload('addressProvider', 'mars_address_provider.wasm')
    await deployer.upload('incentives', 'mars_incentives.wasm')
    await deployer.upload('oracle', `mars_oracle_${config.chainName}.wasm`)
    await deployer.upload('rewardsCollector', `mars_rewards_collector_${config.chainName}.wasm`)

    // Instantiate contracts
    await deployer.setOwnerAddr()
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle()
    await deployer.instantiateRewards()
    await deployer.saveDeploymentAddrsToFile()

    // setup
    await deployer.updateAddressProvider()
    await deployer.initializeAsset(osmoAsset)
    await deployer.initializeAsset(atomAsset)
    await deployer.setOraclePrice(atomOracle)
    await deployer.setOraclePrice(osmoOracle)

    // execute actions
    // printYellow('Testing...')
    // await deployer.executeDeposit()
    // await deployer.executeBorrow()
    // await deployer.executeRepay()
    // await deployer.executeWithdraw()
    // await deployer.executeRewardsSwap()
    // printGreen('ALL TESTS HAVE BEEN SUCCESSFUL')

    // // update owner to multisig address
    // await deployer.updateIncentivesContractOwner()
    // await deployer.updateRedBankContractOwner()
    // await deployer.updateOracleContractOwner()
    // await deployer.updateRewardsContractOwner()
    // await deployer.updateAddressProviderContractOwner()
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
