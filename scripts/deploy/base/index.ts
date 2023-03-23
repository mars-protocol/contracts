import { setupDeployer } from './setupDeployer'
import { DeploymentConfig } from '../../types/config'
import { printGreen, printRed } from '../../utils/chalk'
import { atomOracle, axlUSDCOracle } from '../osmosis/config'

export const taskRunner = async (config: DeploymentConfig) => {
  const deployer = await setupDeployer(config)

  try {
    await deployer.saveStorage()
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('oracle', `mars_oracle_${config.chainName}.wasm`)
    // TODO: upload swapper contract

    // Instantiate contracts
    deployer.setOwnerAddr()
    await deployer.instantiateOracle()
    // TODO: instantiate swapper contract

    // setup
    await deployer.setOracle(atomOracle)
    if (config.mainnet) {
      await deployer.setOracle(axlUSDCOracle)
    }
    // TODO: setup contract

    //run tests
    if (config.runTests) {
      // TODO: run tests
    }

    if (config.multisigAddr) {
      await deployer.updateOracleContractOwner()
      // TODO: transfer swapper contract ownership to multisig
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
