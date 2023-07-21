import { setupDeployer } from '../base/setupDeployer'
import { neutronTestnetConfig } from './config_testnet'

async function main() {
  const deployer = await setupDeployer(neutronTestnetConfig)

  await deployer.recordTwapSnapshots(['untrn'])
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
