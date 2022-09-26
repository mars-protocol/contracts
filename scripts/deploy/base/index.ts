import { setupDeployer } from './setupDeployer'
import { DeploymentConfig, MultisigConfig } from '../../types/config'
import { printRed } from '../../utils/chalk'
import { atomAsset, osmoAsset } from '../osmosis/config'

export const taskRunner = async (config: DeploymentConfig, multisig: MultisigConfig) => {
  const deployer = await setupDeployer(config, multisig)

  try {
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('redBank', 'mars_red_bank.wasm')
    await deployer.upload('addressProvider', 'mars_address_provider.wasm')
    await deployer.upload('incentives', 'mars_incentives.wasm')
    await deployer.upload('oracle', `mars_oracle_${config.chainName}.wasm`)
    await deployer.upload('rewardsCollector', `mars_rewards_collector_${config.chainName}.wasm`)

    // Instantiate contracts
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
    await deployer.setOraclePrice()

    //execute actions
    await deployer.executeDeposit()
    await deployer.executeBorrow()
    await deployer.executeRepay()
    await deployer.executeWithdraw()

    //update owner to multisig address
    await deployer.updateIncentivesContractOwner()
    await deployer.updateRedBankContractOwner()
    await deployer.updateOracleContractOwner()
    await deployer.updateRewardsContractOwner()
    await deployer.updateAddressProviderContractOwner()
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
