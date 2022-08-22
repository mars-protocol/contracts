var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { LocalTerra, MnemonicKey } from '@terra-money/terra.js';
import { join } from 'path';
import { strictEqual } from 'assert';
import 'dotenv/config.js';
import { deployContract, executeContract, Logger, queryContract, setTimeoutDuration, toEncodedBinary, uploadContract, } from '../helpers.js';
import { getBlockHeight, mintCw20, queryBalanceCw20, transferCw20, waitUntilBlockHeight, castVote, } from './test_helpers.js';
// CONSTS
// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH;
const PROPOSAL_EFFECTIVE_DELAY = 5;
const PROPOSAL_REQUIRED_DEPOSIT = 100000000;
const PROPOSAL_VOTING_PERIOD = 10;
// require almost all of the xMars voting power to vote, in order to test that xMars balances at the
// block before proposals were submitted are used
const PROPOSAL_REQUIRED_QUORUM = 0.99;
const ALICE_XMARS_BALANCE = 2000000000;
const ALICE_PROPOSAL_DEPOSIT = PROPOSAL_REQUIRED_DEPOSIT;
const BOB_XMARS_BALANCE = 1000000000;
const BOB_PROPOSAL_DEPOSIT = PROPOSAL_REQUIRED_DEPOSIT + 5000000;
const LUNA_USD_PRICE = 25;
// HELPERS
function assertXmarsBalanceAt(terra, xMars, address, block, expectedBalance) {
    return __awaiter(this, void 0, void 0, function* () {
        const xMarsBalance = yield queryContract(terra, xMars, { balance_at: { address, block } });
        strictEqual(parseInt(xMarsBalance.balance), expectedBalance);
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    // addresses
    const deployer = terra.wallets.test1;
    const alice = terra.wallets.test2;
    const bob = terra.wallets.test3;
    const carol = terra.wallets.test4;
    // mock contract address
    const incentives = new MnemonicKey().accAddress;
    const staking = new MnemonicKey().accAddress;
    console.log('upload contracts');
    const addressProvider = yield deployContract(terra, deployer, '../artifacts/mars_address_provider.wasm', {
        owner: deployer.key.accAddress,
    });
    const council = yield deployContract(terra, deployer, '../artifacts/mars_council.wasm', {
        config: {
            address_provider_address: addressProvider,
            proposal_voting_period: PROPOSAL_VOTING_PERIOD,
            proposal_effective_delay: PROPOSAL_EFFECTIVE_DELAY,
            proposal_expiration_period: 3000,
            proposal_required_deposit: String(PROPOSAL_REQUIRED_DEPOSIT),
            proposal_required_quorum: String(PROPOSAL_REQUIRED_QUORUM),
            proposal_required_threshold: '0.5',
        },
    });
    const oracle = yield deployContract(terra, deployer, '../artifacts/mars_oracle.wasm', { owner: council });
    const maTokenCodeId = yield uploadContract(terra, deployer, '../artifacts/mars_ma_token.wasm');
    const redBank = yield deployContract(terra, deployer, '../artifacts/mars_red_bank.wasm', {
        config: {
            owner: council,
            address_provider_address: addressProvider,
            safety_fund_fee_share: '0.1',
            treasury_fee_share: '0.2',
            ma_token_code_id: maTokenCodeId,
            close_factor: '0.5',
        },
    });
    const vesting = yield deployContract(terra, deployer, '../artifacts/mars_vesting.wasm', {
        address_provider_address: addressProvider,
        unlock_schedule: {
            start_time: 1893452400,
            cliff: 15552000,
            duration: 94608000, // 3 years
        },
    });
    const mars = yield deployContract(terra, deployer, join(CW_PLUS_ARTIFACTS_PATH, 'cw20_base.wasm'), {
        name: 'Mars',
        symbol: 'MARS',
        decimals: 6,
        initial_balances: [],
        mint: { minter: deployer.key.accAddress },
    });
    const xMars = yield deployContract(terra, deployer, '../artifacts/mars_xmars_token.wasm', {
        name: 'xMars',
        symbol: 'xMARS',
        decimals: 6,
        initial_balances: [],
        mint: { minter: deployer.key.accAddress },
    });
    // update address provider
    yield executeContract(terra, deployer, addressProvider, {
        update_config: {
            config: {
                owner: deployer.key.accAddress,
                council_address: council,
                incentives_address: incentives,
                mars_token_address: mars,
                oracle_address: oracle,
                red_bank_address: redBank,
                staking_address: staking,
                vesting_address: vesting,
                xmars_token_address: xMars,
                protocol_admin_address: deployer.key.accAddress,
            },
        },
    }, { logger: logger });
    // mint tokens
    yield mintCw20(terra, deployer, mars, alice.key.accAddress, ALICE_PROPOSAL_DEPOSIT, logger);
    yield mintCw20(terra, deployer, mars, bob.key.accAddress, BOB_PROPOSAL_DEPOSIT, logger);
    yield mintCw20(terra, deployer, xMars, alice.key.accAddress, ALICE_XMARS_BALANCE, logger);
    yield mintCw20(terra, deployer, xMars, bob.key.accAddress, BOB_XMARS_BALANCE, logger);
    // TESTS
    console.log('alice submits a proposal to initialise a new asset in the red bank');
    let txResult = yield executeContract(terra, alice, mars, {
        send: {
            contract: council,
            amount: String(ALICE_PROPOSAL_DEPOSIT),
            msg: toEncodedBinary({
                submit_proposal: {
                    title: 'Init Luna',
                    description: 'Initialise Luna',
                    link: 'http://www.terra.money',
                    messages: [
                        // init luna as an asset in the red bank contract
                        {
                            execution_order: 1,
                            msg: {
                                wasm: {
                                    execute: {
                                        contract_addr: redBank,
                                        funds: [],
                                        msg: toEncodedBinary({
                                            init_asset: {
                                                asset: { native: { denom: 'uluna' } },
                                                asset_params: {
                                                    initial_borrow_rate: '0.1',
                                                    max_loan_to_value: '0.55',
                                                    reserve_factor: '0.2',
                                                    liquidation_threshold: '0.65',
                                                    liquidation_bonus: '0.1',
                                                    interest_rate_model_params: {
                                                        dynamic: {
                                                            min_borrow_rate: '0.0',
                                                            max_borrow_rate: '2.0',
                                                            kp_1: '0.02',
                                                            optimal_utilization_rate: '0.7',
                                                            kp_augmentation_threshold: '0.15',
                                                            kp_2: '0.05',
                                                            update_threshold_txs: 5,
                                                            update_threshold_seconds: 600,
                                                        },
                                                    },
                                                    active: true,
                                                    deposit_enabled: true,
                                                    borrow_enabled: true,
                                                },
                                            },
                                        }),
                                    },
                                },
                            },
                        },
                        // set a fixed price for luna in the oracle contract
                        {
                            execution_order: 2,
                            msg: {
                                wasm: {
                                    execute: {
                                        contract_addr: oracle,
                                        funds: [],
                                        msg: toEncodedBinary({
                                            set_asset: {
                                                asset: { native: { denom: 'uluna' } },
                                                price_source: { fixed: { price: String(LUNA_USD_PRICE) } },
                                            },
                                        }),
                                    },
                                },
                            },
                        },
                    ],
                },
            }),
        },
    }, { logger: logger });
    let blockHeight = yield getBlockHeight(terra, txResult);
    const aliceProposalVotingPeriodEnd = blockHeight + PROPOSAL_VOTING_PERIOD;
    const aliceProposalEffectiveDelayEnd = aliceProposalVotingPeriodEnd + PROPOSAL_EFFECTIVE_DELAY;
    const aliceProposalId = parseInt(txResult.logs[0].eventsByType.wasm.proposal_id[0]);
    console.log('bob submits a proposal that will fail');
    txResult = yield executeContract(terra, bob, mars, {
        send: {
            contract: council,
            amount: String(BOB_PROPOSAL_DEPOSIT),
            msg: toEncodedBinary({
                submit_proposal: {
                    title: 'Null',
                    description: 'An empty proposal',
                    execute_calls: [],
                },
            }),
        },
    }, { logger: logger });
    blockHeight = yield getBlockHeight(terra, txResult);
    const bobProposalVotingPeriodEnd = blockHeight + PROPOSAL_VOTING_PERIOD;
    const bobProposalId = parseInt(txResult.logs[0].eventsByType.wasm.proposal_id[0]);
    console.log('alice sends entire xMars balance to bob');
    yield transferCw20(terra, alice, xMars, bob.key.accAddress, ALICE_XMARS_BALANCE, logger);
    yield assertXmarsBalanceAt(terra, xMars, alice.key.accAddress, blockHeight - 1, ALICE_XMARS_BALANCE);
    yield assertXmarsBalanceAt(terra, xMars, bob.key.accAddress, blockHeight - 1, BOB_XMARS_BALANCE);
    console.log('mint a large amount of xMars to carol');
    // proposal quorum should use xMars balances from the blockHeight before a proposal was submitted.
    // so, proposal quorum should still be reached by alice's and bob's votes, even after a large
    // amount of xMars is minted to carol.
    yield mintCw20(terra, deployer, xMars, carol.key.accAddress, ALICE_XMARS_BALANCE * BOB_XMARS_BALANCE * 100, logger);
    yield assertXmarsBalanceAt(terra, xMars, carol.key.accAddress, blockHeight - 1, 0);
    console.log('vote');
    yield castVote(terra, alice, council, aliceProposalId, 'for', logger);
    yield castVote(terra, bob, council, aliceProposalId, 'against', logger);
    console.log('wait for voting periods to end');
    yield waitUntilBlockHeight(terra, Math.max(aliceProposalVotingPeriodEnd, bobProposalVotingPeriodEnd));
    console.log('end proposals');
    console.log("- alice's proposal passes, so her Mars deposit is returned");
    const aliceMarsBalanceBefore = yield queryBalanceCw20(terra, alice.key.accAddress, mars);
    yield executeContract(terra, deployer, council, { end_proposal: { proposal_id: aliceProposalId } }, { logger: logger });
    const aliceProposalStatus = yield queryContract(terra, council, { proposal: { proposal_id: aliceProposalId } });
    strictEqual(aliceProposalStatus.status, 'passed');
    const aliceMarsBalanceAfter = yield queryBalanceCw20(terra, alice.key.accAddress, mars);
    strictEqual(aliceMarsBalanceAfter, aliceMarsBalanceBefore + ALICE_PROPOSAL_DEPOSIT);
    console.log("- bob's proposal was rejected, so his Mars deposit is sent to the staking contract");
    const bobMarsBalanceBefore = yield queryBalanceCw20(terra, bob.key.accAddress, mars);
    const stakingContractMarsBalanceBefore = yield queryBalanceCw20(terra, staking, mars);
    yield executeContract(terra, deployer, council, { end_proposal: { proposal_id: bobProposalId } }, { logger: logger });
    const bobProposalStatus = yield queryContract(terra, council, { proposal: { proposal_id: bobProposalId } });
    strictEqual(bobProposalStatus.status, 'rejected');
    const bobMarsBalanceAfter = yield queryBalanceCw20(terra, bob.key.accAddress, mars);
    const stakingContractMarsBalanceAfter = yield queryBalanceCw20(terra, staking, mars);
    strictEqual(bobMarsBalanceAfter, bobMarsBalanceBefore);
    strictEqual(stakingContractMarsBalanceAfter, stakingContractMarsBalanceBefore + BOB_PROPOSAL_DEPOSIT);
    console.log('wait for effective delay period to end');
    yield waitUntilBlockHeight(terra, aliceProposalEffectiveDelayEnd);
    console.log('execute proposal');
    yield executeContract(terra, deployer, council, { execute_proposal: { proposal_id: aliceProposalId } }, { logger: logger });
    // check that the asset has been initialised on the red bank
    const marketsList = yield queryContract(terra, redBank, { markets_list: {} });
    strictEqual(marketsList.markets_list[0].denom, 'uluna');
    // check that the asset has been initialised in the oracle contract
    const assetConfig = yield queryContract(terra, oracle, {
        asset_price_source: { asset: { native: { denom: 'uluna' } } },
    });
    strictEqual(parseInt(assetConfig.fixed.price), LUNA_USD_PRICE);
    console.log('OK');
    logger.showGasConsumption();
}))();
