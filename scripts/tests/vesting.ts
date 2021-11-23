import {
  LocalTerra,
  MnemonicKey
} from "@terra-money/terra.js";
import { join } from "path";
import { strictEqual } from "assert";
import "dotenv/config.js";
import {
  deployContract,
  executeContract,
  queryContract,
  toEncodedBinary,
  uploadContract,
} from "../helpers.js";
import {
  getBlockHeight,
  mintCw20,
  queryBalanceCw20
} from "./test_helpers.js";

// CONSTS

// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!;
const ASTROPORT_ARTIFACTS_PATH = process.env.ASTROPORT_ARTIFACTS_PATH!;

// staking parameters
const COOLDOWN_DURATION_SECONDS = 2;

// council parameters
const PROPOSAL_EFFECTIVE_DELAY = 5;
const PROPOSAL_REQUIRED_DEPOSIT = 100_000000;
const PROPOSAL_VOTING_PERIOD = 10;
// require almost all of the xMars voting power to vote, in order to test that xMars balances at the
// block before proposals were submitted are used
const PROPOSAL_REQUIRED_QUORUM = 0.99;

const ALICE_MARS_BALANCE = PROPOSAL_REQUIRED_DEPOSIT;
const BOB_WALLET_MARS_BALANCE = 12345; // Mars tokens in bob's wallet
const BOB_VESTING_MARS_BALANCE = 1_000_000000; // Mars tokens allocated to bob in the vesting contract

// MAIN

(async () => {
  const terra = new LocalTerra();

  // addresses
  const deployer = terra.wallets.test1;
  const alice = terra.wallets.test2; // a user who creates a governance proposal
  const bob = terra.wallets.test3; // a person receiving a MARS token allocations
  const jv = terra.wallets.test4; // Mars JV
  // mock contract addresses
  const astroportGenerator = new MnemonicKey().accAddress

  console.log("deployer:", deployer.key.accAddress);
  console.log("alice:   ", alice.key.accAddress);
  console.log("bob:     ", bob.key.accAddress);
  console.log("jv:      ", jv.key.accAddress);

  process.stdout.write("deploying astroport... ");

  const tokenCodeID = await uploadContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_token.wasm")
  );
  const pairCodeID = await uploadContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_pair.wasm")
  );
  const astroportFactory = await deployContract(
    terra,
    deployer,
    join(ASTROPORT_ARTIFACTS_PATH, "astroport_factory.wasm"),
    {
      token_code_id: tokenCodeID,
      generator_address: astroportGenerator,
      pair_configs: [
        {
          code_id: pairCodeID,
          pair_type: { xyk: {} },
          total_fee_bps: 0,
          maker_fee_bps: 0,
        },
      ],
    }
  );

  console.log("done!");

  process.stdout.write("deploying address provider... ");

  const addressProvider = await deployContract(
    terra,
    deployer,
    "../artifacts/mars_address_provider.wasm",
    {
      owner: deployer.key.accAddress,
    }
  );

  console.log("done!");

  process.stdout.write("deploying council... ");

  const council = await deployContract(
    terra,
    deployer,
    "../artifacts/mars_council.wasm",
    {
      config: {
        address_provider_address: addressProvider,
        proposal_voting_period: PROPOSAL_VOTING_PERIOD,
        proposal_effective_delay: PROPOSAL_EFFECTIVE_DELAY,
        proposal_expiration_period: 3000,
        proposal_required_deposit: String(PROPOSAL_REQUIRED_DEPOSIT),
        proposal_required_quorum: String(PROPOSAL_REQUIRED_QUORUM),
        proposal_required_threshold: "0.05",
      },
    }
  );

  console.log("done!");

  process.stdout.write("deploying staking... ");

  const staking = await deployContract(
    terra,
    deployer,
    "../artifacts/mars_staking.wasm",
    {
      config: {
        owner: deployer.key.accAddress,
        address_provider_address: addressProvider,
        astroport_factory_address: astroportFactory,
        astroport_max_spread: "0.05",
        cooldown_duration: COOLDOWN_DURATION_SECONDS,
      },
    }
  );

  console.log("done!");

  process.stdout.write("deploying vesting... ");

  const vesting = await deployContract(
    terra,
    deployer,
    "../artifacts/mars_vesting.wasm",
    {
      address_provider_address: addressProvider,
      default_unlock_schedule: {
        start_time: 0,
        cliff: 0,
        duration: 0,
      },
    }
  );

  console.log("done!");

  process.stdout.write("deploying mars token... ");

  const mars = await deployContract(
    terra,
    deployer,
    join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm"),
    {
      name: "Mars",
      symbol: "MARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: deployer.key.accAddress },
    }
  );

  console.log("done!");

  process.stdout.write("deploying xmars token... ");

  const xMars = await deployContract(
    terra,
    deployer,
    "../artifacts/mars_xmars_token.wasm",
    {
      name: "xMars",
      symbol: "xMARS",
      decimals: 6,
      initial_balances: [],
      mint: { minter: staking },
    }
  );

  console.log("done!");

  process.stdout.write("updating address provider... ");

  await executeContract(terra, deployer, addressProvider, {
    update_config: {
      config: {
        owner: deployer.key.accAddress,
        council_address: council,
        mars_token_address: mars,
        staking_address: staking,
        vesting_address: vesting,
        xmars_token_address: xMars,
        protocol_admin_address: jv.key.accAddress,
      },
    },
  });

  console.log("done!");

  process.stdout.write("mint Mars tokens for alice and JV... ");

  await mintCw20(terra, deployer, mars, alice.key.accAddress, ALICE_MARS_BALANCE);
  await mintCw20(terra, deployer, mars, bob.key.accAddress, BOB_WALLET_MARS_BALANCE);
  await mintCw20(terra, deployer, mars, jv.key.accAddress, BOB_VESTING_MARS_BALANCE);

  console.log("done!");

  // TESTS

  {
    process.stdout.write(
      "bob stakes Mars available in his wallet and receives the same amount of xMars... "
    );

    await executeContract(terra, bob, mars, {
      send: {
        contract: staking,
        amount: String(BOB_WALLET_MARS_BALANCE),
        msg: toEncodedBinary({ stake: {} }),
      },
    });

    console.log("success!");
  }

  {
    process.stdout.write("JV creates an allocation for bob... ");

    await executeContract(terra, jv, mars, {
      send: {
        contract: vesting,
        amount: String(BOB_VESTING_MARS_BALANCE),
        msg: toEncodedBinary({
          create_allocations: {
            allocations: [
              [
                bob.key.accAddress,
                {
                  amount: String(BOB_VESTING_MARS_BALANCE),
                  vest_schedule: {
                    start_time: 0,
                    cliff: 0,
                    duration: 0,
                  },
                  unlock_schedule: null,
                },
              ],
            ],
          },
        }),
      },
    });

    console.log("success!");
  }

  {
    process.stdout.write(
      "bob stakes Mars through vesting contract and receives the same amount of xMars... "
    );

    const txResult = await executeContract(terra, bob, vesting, {
      stake: {},
    });

    console.log("success!");

    process.stdout.write(
      "contract should return correct voting power before and after the staking event... "
    );

    // the block height where bob performed the staking action
    const height = await getBlockHeight(terra, txResult);

    // before the height, bob should have 0 voting power
    const votingPowerBefore: string = await queryContract(terra, vesting, {
      voting_power_at: {
        account: bob.key.accAddress,
        block: height - 1,
      },
    });
    strictEqual(votingPowerBefore, "0");

    // at or after the height, bob should have `BOB_VESTING_MARS_BALANCE` voting power
    const votingPowerAfter: string = await queryContract(terra, vesting, {
      voting_power_at: {
        account: bob.key.accAddress,
        block: height,
      },
    });
    strictEqual(votingPowerAfter, String(BOB_VESTING_MARS_BALANCE));

    console.log("success!");
  }

  {
    process.stdout.write("alice submits a governance proposal... ");

    const submitProposalResult = await executeContract(terra, alice, mars, {
      send: {
        contract: council,
        amount: String(ALICE_MARS_BALANCE),
        msg: toEncodedBinary({
          submit_proposal: {
            title: "Test",
            description: "This is a test",
            link: "https://twitter.com/",
            messages: [],
          },
        }),
      },
    });
    const proposalId = parseInt(
      submitProposalResult.logs[0].eventsByType.wasm.proposal_id[0]
    );

    console.log("success!");

    process.stdout.write("bob votes for the governance proposal... ");

    const castVoteResult = await executeContract(terra, bob, council, {
      cast_vote: {
        proposal_id: proposalId,
        vote: "for",
      },
    });

    console.log("success!");

    process.stdout.write("council correctly calculates bob's total voting power... ");

    const bobVotingPower = parseInt(
      castVoteResult.logs[0].eventsByType.wasm.voting_power[0]
    );
    strictEqual(bobVotingPower, BOB_WALLET_MARS_BALANCE + BOB_VESTING_MARS_BALANCE);

    console.log("success!");
  }

  {
    process.stdout.write("bob withdraws xMars... ");

    await executeContract(terra, bob, vesting, {
      withdraw: {},
    });

    const bobXMarsBalance = await queryBalanceCw20(terra, bob.key.accAddress, xMars);
    strictEqual(bobXMarsBalance, BOB_WALLET_MARS_BALANCE + BOB_VESTING_MARS_BALANCE);

    console.log("success!");
  }

  console.log("OK");
})();
