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
    await deployer.upload('accountNft', wasmFile('mars_account_nft'))
    await deployer.upload('mockVault', wasmFile('mars_mock_vault'))
    await deployer.upload('marsOracleAdapter', wasmFile('mars_oracle_adapter'))
    await deployer.upload('swapper', wasmFile(swapperContractName))
    await deployer.upload('creditManager', wasmFile('mars_credit_manager'))

    // Instantiate contracts
    await deployer.instantiateMockVault()
    await deployer.instantiateMarsOracleAdapter()
    await deployer.instantiateSwapper()
    await deployer.instantiateCreditManager()
    await deployer.instantiateNftContract()
    await deployer.transferNftContractOwnership()
    await deployer.saveDeploymentAddrsToFile()

    // Test basic user flows
    if (config.testActions) {
      await deployer.grantCreditLines()
      await deployer.setupOraclePrices()
      await deployer.setupRedBankMarketsForZapDenoms()
      const rover = await deployer.newUserRoverClient(config.testActions)
      await rover.createCreditAccount()
      await rover.deposit()
      await rover.borrow()
      await rover.swap()
      await rover.repay()
      await rover.withdraw()

      const vaultConfig = config.vaults[0]
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

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
