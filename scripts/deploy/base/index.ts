import { setupDeployer } from './setupDeployer'
import { printRed, printYellow } from '../../utils/chalk'
import { DeploymentConfig, VaultType } from '../../types/config'
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
    await deployer.upload('mockVault', wasmFile('mock_vault'))
    await deployer.upload('marsOracleAdapter', wasmFile('mars_oracle_adapter'))
    await deployer.upload('swapper', wasmFile(swapperContractName))
    await deployer.upload('mockZapper', wasmFile('mock_zapper'))
    await deployer.upload('creditManager', wasmFile('credit_manager'))

    // Instantiate contracts
    await deployer.instantiateNftContract()
    await deployer.instantiateMockVault()
    await deployer.instantiateMarsOracleAdapter()
    await deployer.instantiateSwapper()
    await deployer.instantiateZapper()
    await deployer.instantiateCreditManager()
    await deployer.transferNftContractOwnership()
    await deployer.grantCreditLines()
    await deployer.setupOraclePricesForZapDenoms()
    await deployer.setupRedBankMarketsForZapDenoms()
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

    await rover.zap()
    await rover.unzap()

    await rover.vaultDeposit()
    if (config.vaultType === VaultType.UNLOCKED) {
      await rover.vaultWithdraw()
    } else {
      await rover.vaultRequestUnlock()
    }

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
