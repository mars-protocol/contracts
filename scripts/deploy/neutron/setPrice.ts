import { setupDeployer } from '../base/setupDeployer'
import { neutronTestnetConfig, atomOracle } from './config_testnet'

async function main() {
  const deployer = await setupDeployer(neutronTestnetConfig)

  await deployer.setOracle(atomOracle)
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
