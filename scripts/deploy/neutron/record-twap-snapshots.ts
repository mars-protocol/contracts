import { setupDeployer } from '../base/setup-deployer'
import { neutronTestnetConfig } from './testnet-config'

async function main() {
  const deployer = await setupDeployer(neutronTestnetConfig, '')

  await deployer.recordTwapSnapshots(['untrn'])
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
