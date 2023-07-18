import { setupDeployer } from '../base/setupDeployer'
import { neutronTestnetConfig, atomOracleTestnet } from './config'

async function main() {
  const deployer = await setupDeployer(neutronTestnetConfig)

  await deployer.setOracle(atomOracleTestnet)
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
