import { setupDeployer } from './setupDeployer'
import { printRed, printYellow } from '../../utils/chalk'
import { DeploymentConfig } from '../../types/config'
import { wasmFile } from '../../utils/environment'

export interface TaskRunnerProps {
  config: DeploymentConfig
  swapperContractName: string
}

export const taskRunner = async ({ config, swapperContractName }: TaskRunnerProps) => {
  const deployer = await setupDeployer(config)
  try {
    // Upload contracts
    await deployer.upload('accountNft', wasmFile('account_nft'))
    await deployer.upload('mockRedBank', wasmFile('mock_red_bank'))
    await deployer.upload('mockVault', wasmFile('mock_vault'))
    await deployer.upload('mockOracle', wasmFile('mock_oracle'))
    await deployer.upload('swapper', wasmFile(swapperContractName))
    await deployer.upload('creditManager', wasmFile('credit_manager'))

    // Instantiate contracts
    await deployer.instantiateNftContract()
    await deployer.instantiateMockRedBank()
    await deployer.instantiateMockOracle()
    await deployer.instantiateMockVault()
    await deployer.instantiateSwapper()
    await deployer.instantiateCreditManager()
    await deployer.transferNftContractOwnership()
    await deployer.saveDeploymentAddrsToFile()

    const rover = await deployer.newUserRoverClient()

    // Test basic user flows
    await rover.createCreditAccount()
    await rover.deposit()
    await rover.borrow()
    await rover.repay()
    // TODO: Osmosis-bindings need updating
    // await rover.swap()
    await rover.withdraw()

    // TODO: Use after token factory is launched and integrated into mock_vault
    //       or Apollo vaults are on testnet
    // await rover.vaultDeposit()

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
