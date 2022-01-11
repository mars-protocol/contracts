import {
  LCDClient,
  LocalTerra,
  MnemonicKey,
  Wallet
} from "@terra-money/terra.js"
import { join } from "path"
import { strictEqual } from "assert"
import 'dotenv/config.js'
import {
  deployContract,
  executeContract, instantiateContract, Logger,
  queryContract,
  setTimeoutDuration,
  sleep,
  toEncodedBinary,
  uploadContract
} from "../helpers.js"
import {
  getBlockHeight,
  mintCw20,
} from "./test_helpers.js"

// CONSTS

// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!
const MARS_MOCKS_ARTIFACTS_PATH = process.env.MARS_MOCKS_ARTIFACTS_PATH!

const PROPOSAL_EFFECTIVE_DELAY = 5
const PROPOSAL_REQUIRED_DEPOSIT = 100_000000
const PROPOSAL_VOTING_PERIOD = 10
const PROPOSAL_REQUIRED_QUORUM = 0.80

const JOHN_XMARS_BALANCE = 2_000_000000
const JOHN_PROPOSAL_DEPOSIT = PROPOSAL_REQUIRED_DEPOSIT

// HELPERS

async function castVote(
  terra: LCDClient,
  wallet: Wallet,
  council: string,
  proposalId: number,
  vote: string,
  logger?: Logger
) {
  return await executeContract(terra, wallet, council,
    {
      cast_vote: {
        proposal_id: proposalId,
        vote
      }
    },
    { logger: logger }
  )
}

async function waitUntilBlockHeight(
  terra: LCDClient,
  blockHeight: number,
) {
  const maxTries = 10
  let tries = 0
  let backoff = 1
  while (true) {
    const latestBlock = await terra.tendermint.blockInfo()
    const latestBlockHeight = parseInt(latestBlock.block.header.height)

    if (latestBlockHeight >= blockHeight) {
      break
    }

    // timeout
    tries++
    if (tries == maxTries) {
      throw new Error(
        `timed out waiting for block height ${blockHeight}, current block height: ${latestBlockHeight}`
      )
    }

    // exponential backoff
    await sleep(backoff * 1000)
    backoff *= 2
  }
}

// MAIN

(async () => {
  setTimeoutDuration(0)

  const logger = new Logger()

  const terra = new LocalTerra()

  // addresses
  const deployer = terra.wallets.test1
  const john = terra.wallets.test2
  // mock contract addresses
  const staking = new MnemonicKey().accAddress

  console.log("upload contracts")

  const addressProvider = await deployContract(terra, deployer, "../artifacts/mars_address_provider.wasm",
    { owner: deployer.key.accAddress }
  )

  const council = await deployContract(terra, deployer, "../artifacts/mars_council.wasm",
    {
      config: {
        address_provider_address: addressProvider,
        proposal_voting_period: PROPOSAL_VOTING_PERIOD,
        proposal_effective_delay: PROPOSAL_EFFECTIVE_DELAY,
        proposal_expiration_period: 3000,
        proposal_required_deposit: String(PROPOSAL_REQUIRED_DEPOSIT),
        proposal_required_quorum: String(PROPOSAL_REQUIRED_QUORUM),
        proposal_required_threshold: "0.05"
      }
    }
  )

  const vesting = await deployContract(terra, deployer, "../artifacts/mars_vesting.wasm",
    {
      address_provider_address: addressProvider,
      unlock_start_time: 0,
      unlock_cliff: 0,
      unlock_duration: 0
    }
  )

  const mars = await deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm"),
    {
      name: "Mars",
      symbol: "MARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress },
    }
  )

  const xMars = await deployContract(terra, deployer, "../artifacts/mars_xmars_token.wasm",
    {
      name: "xMars",
      symbol: "xMARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress },
    }
  )

  // update address provider
  await executeContract(terra, deployer, addressProvider,
    {
      update_config: {
        config: {
          owner: deployer.key.accAddress,
          council_address: council,
          vesting_address: vesting,
          mars_token_address: mars,
          xmars_token_address: xMars,
          staking_address: staking,
        }
      }
    },
    { logger: logger }
  )

  // mint tokens
  await mintCw20(terra, deployer, mars, john.key.accAddress, JOHN_PROPOSAL_DEPOSIT, logger)
  await mintCw20(terra, deployer, xMars, john.key.accAddress, JOHN_XMARS_BALANCE, logger)

  // deploy `counter_version_one` with admin set to council
  const counterVer1CodeId = await uploadContract(terra, deployer, join(MARS_MOCKS_ARTIFACTS_PATH, "counter_version_one.wasm"))
  const counterVer1 = await instantiateContract(terra, deployer, counterVer1CodeId, { owner: deployer.key.accAddress }, { admin: council })

  // upload `counter_version_two`
  const counterVer2CodeId = await uploadContract(terra, deployer, join(MARS_MOCKS_ARTIFACTS_PATH, "counter_version_two.wasm"))

  // TESTS

  console.log("verify first version of `counter` contract")

  await executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger })
  await executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger })

  const countResponse = await queryContract(terra, counterVer1, { get_count: {} })
  strictEqual(countResponse.count, 2)

  const versionResponse = await queryContract(terra, counterVer1, { get_version: {} })
  strictEqual(versionResponse.version, "one")

  console.log("john submits a proposal to initialise `counter` contract migration")

  let txResult = await executeContract(terra, john, mars,
    {
      send: {
        contract: council,
        amount: String(JOHN_PROPOSAL_DEPOSIT),
        msg: toEncodedBinary({
          submit_proposal: {
            title: "Migrate counter contract",
            description: "Migrate counter_version_one -> counter_version_two",
            link: "http://www.terra.money",
            messages: [
              {
                execution_order: 1,
                msg: {
                  wasm: {
                    migrate: {
                      contract_addr: counterVer1,
                      new_code_id: counterVer2CodeId,
                      msg: toEncodedBinary({})
                    }
                  }
                }
              },
            ]
          }
        })
      }
    },
    { logger: logger }
  )
  let blockHeight = await getBlockHeight(terra, txResult)
  const johnProposalVotingPeriodEnd = blockHeight + PROPOSAL_VOTING_PERIOD
  const johnProposalEffectiveDelayEnd = johnProposalVotingPeriodEnd + PROPOSAL_EFFECTIVE_DELAY
  const johnProposalId = parseInt(txResult.logs[0].eventsByType.wasm.proposal_id[0])

  console.log("vote")

  await castVote(terra, john, council, johnProposalId, "for", logger)

  console.log("wait for voting periods to end")

  await waitUntilBlockHeight(terra, johnProposalVotingPeriodEnd)

  console.log("end proposal")

  await executeContract(terra, deployer, council, { end_proposal: { proposal_id: johnProposalId } }, { logger: logger })

  const johnProposalStatus = await queryContract(terra, council, { proposal: { proposal_id: johnProposalId } })
  strictEqual(johnProposalStatus.status, "passed")

  console.log("wait for effective delay period to end")

  await waitUntilBlockHeight(terra, johnProposalEffectiveDelayEnd)

  console.log("execute proposal")

  await executeContract(terra, deployer, council, { execute_proposal: { proposal_id: johnProposalId } }, { logger: logger })

  console.log("verify second version of `counter` contract")

  await executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger })

  const countResponse2 = await queryContract(terra, counterVer1, { get_count: {} })
  strictEqual(countResponse2.count, 3)

  const versionResponse2 = await queryContract(terra, counterVer1, { get_version: {} })
  strictEqual(versionResponse2.version, "two")

  console.log("OK")

  logger.showGasConsumption()
})()
