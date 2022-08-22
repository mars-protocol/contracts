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
import { deployContract, executeContract, instantiateContract, updateContractAdmin, Logger, queryContract, setTimeoutDuration, toEncodedBinary, uploadContract, } from '../helpers.js';
import { getBlockHeight, mintCw20, waitUntilBlockHeight } from './test_helpers.js';
// CONSTS
// required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH;
const MARS_MOCKS_ARTIFACTS_PATH = process.env.MARS_MOCKS_ARTIFACTS_PATH;
const PROPOSAL_EFFECTIVE_DELAY = 5;
const PROPOSAL_REQUIRED_DEPOSIT = 100000000;
const PROPOSAL_VOTING_PERIOD = 10;
const PROPOSAL_REQUIRED_QUORUM = 0.8;
const JOHN_XMARS_BALANCE = 2000000000;
const JOHN_PROPOSAL_DEPOSIT = PROPOSAL_REQUIRED_DEPOSIT;
// HELPERS
function castVote(terra, wallet, council, proposalId, vote, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, council, {
            cast_vote: {
                proposal_id: proposalId,
                vote,
            },
        }, { logger: logger });
    });
}
// MAIN
(() => __awaiter(void 0, void 0, void 0, function* () {
    setTimeoutDuration(0);
    const logger = new Logger();
    const terra = new LocalTerra();
    // addresses
    const deployer = terra.wallets.test1;
    const john = terra.wallets.test2;
    // mock contract addresses
    const staking = new MnemonicKey().accAddress;
    console.log('upload contracts');
    const addressProvider = yield deployContract(terra, deployer, '../artifacts/mars_address_provider.wasm', {
        owner: deployer.key.accAddress,
    });
    const councilCodeId = yield uploadContract(terra, deployer, '../artifacts/mars_council.wasm');
    // instantiate `mars_council` with admin set to deployer
    const council = yield instantiateContract(terra, deployer, councilCodeId, {
        config: {
            address_provider_address: addressProvider,
            proposal_voting_period: PROPOSAL_VOTING_PERIOD,
            proposal_effective_delay: PROPOSAL_EFFECTIVE_DELAY,
            proposal_expiration_period: 3000,
            proposal_required_deposit: String(PROPOSAL_REQUIRED_DEPOSIT),
            proposal_required_quorum: String(PROPOSAL_REQUIRED_QUORUM),
            proposal_required_threshold: '0.5',
        },
    }, { admin: deployer.key.accAddress });
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
                vesting_address: vesting,
                mars_token_address: mars,
                xmars_token_address: xMars,
                staking_address: staking,
            },
        },
    }, { logger: logger });
    // deploy `counter_version_one` with admin set to council
    const counterVer1CodeId = yield uploadContract(terra, deployer, join(MARS_MOCKS_ARTIFACTS_PATH, 'counter_version_one.wasm'));
    const counterVer1 = yield instantiateContract(terra, deployer, counterVer1CodeId, { owner: deployer.key.accAddress }, { admin: council });
    // mint tokens
    yield mintCw20(terra, deployer, mars, john.key.accAddress, JOHN_PROPOSAL_DEPOSIT, logger);
    yield mintCw20(terra, deployer, xMars, john.key.accAddress, JOHN_XMARS_BALANCE, logger);
    // TESTS
    // migrate `counter` contract
    {
        console.log('upload new version of `counter` contract');
        const counterVer2CodeId = yield uploadContract(terra, deployer, join(MARS_MOCKS_ARTIFACTS_PATH, 'counter_version_two.wasm'));
        console.log('verify first version of `counter` contract');
        yield executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger });
        yield executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger });
        const countResponse = yield queryContract(terra, counterVer1, { get_count: {} });
        strictEqual(countResponse.count, 2);
        const versionResponse = yield queryContract(terra, counterVer1, { get_version: {} });
        strictEqual(versionResponse.version, 'one');
        console.log('john submits a proposal to initialise `counter` contract migration');
        let txResult = yield executeContract(terra, john, mars, {
            send: {
                contract: council,
                amount: String(JOHN_PROPOSAL_DEPOSIT),
                msg: toEncodedBinary({
                    submit_proposal: {
                        title: 'Migrate counter contract',
                        description: 'Migrate counter_version_one -> counter_version_two',
                        link: 'http://www.terra.money',
                        messages: [
                            {
                                execution_order: 1,
                                msg: {
                                    wasm: {
                                        migrate: {
                                            contract_addr: counterVer1,
                                            new_code_id: counterVer2CodeId,
                                            msg: toEncodedBinary({}),
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
        const johnProposalVotingPeriodEnd = blockHeight + PROPOSAL_VOTING_PERIOD;
        const johnProposalEffectiveDelayEnd = johnProposalVotingPeriodEnd + PROPOSAL_EFFECTIVE_DELAY;
        const johnProposalId = parseInt(txResult.logs[0].eventsByType.wasm.proposal_id[0]);
        console.log('vote');
        yield castVote(terra, john, council, johnProposalId, 'for', logger);
        console.log('wait for voting periods to end');
        yield waitUntilBlockHeight(terra, johnProposalVotingPeriodEnd);
        console.log('end proposal');
        yield executeContract(terra, deployer, council, { end_proposal: { proposal_id: johnProposalId } }, { logger: logger });
        const johnProposalStatus = yield queryContract(terra, council, { proposal: { proposal_id: johnProposalId } });
        strictEqual(johnProposalStatus.status, 'passed');
        console.log('wait for effective delay period to end');
        yield waitUntilBlockHeight(terra, johnProposalEffectiveDelayEnd);
        console.log('execute proposal');
        yield executeContract(terra, deployer, council, { execute_proposal: { proposal_id: johnProposalId } }, { logger: logger });
        console.log('verify second version of `counter` contract');
        yield executeContract(terra, deployer, counterVer1, { increment: {} }, { logger: logger });
        const countResponse2 = yield queryContract(terra, counterVer1, { get_count: {} });
        strictEqual(countResponse2.count, 3);
        const versionResponse2 = yield queryContract(terra, counterVer1, { get_version: {} });
        strictEqual(versionResponse2.version, 'two');
    }
    // migrate `council` contract
    {
        console.log('upload new version of `council` contract');
        // we use `counter` contract with migrate entrypoint as new version of `council` contract
        const councilVer2CodeId = yield uploadContract(terra, deployer, join(MARS_MOCKS_ARTIFACTS_PATH, 'counter_version_two.wasm'));
        console.log('update council admin to itself');
        yield updateContractAdmin(terra, deployer, council, council);
        console.log('john submits a proposal to initialise `council` contract migration');
        let txResult = yield executeContract(terra, john, mars, {
            send: {
                contract: council,
                amount: String(JOHN_PROPOSAL_DEPOSIT),
                msg: toEncodedBinary({
                    submit_proposal: {
                        title: 'Migrate council contract',
                        description: 'Migrate council -> counter_version_two',
                        link: 'http://www.terra.money',
                        messages: [
                            {
                                execution_order: 1,
                                msg: {
                                    wasm: {
                                        migrate: {
                                            contract_addr: council,
                                            new_code_id: councilVer2CodeId,
                                            msg: toEncodedBinary({}),
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
        const johnProposalVotingPeriodEnd = blockHeight + PROPOSAL_VOTING_PERIOD;
        const johnProposalEffectiveDelayEnd = johnProposalVotingPeriodEnd + PROPOSAL_EFFECTIVE_DELAY;
        const johnProposalId = parseInt(txResult.logs[0].eventsByType.wasm.proposal_id[0]);
        console.log('vote');
        yield castVote(terra, john, council, johnProposalId, 'for', logger);
        console.log('wait for voting periods to end');
        yield waitUntilBlockHeight(terra, johnProposalVotingPeriodEnd);
        console.log('end proposal');
        yield executeContract(terra, deployer, council, { end_proposal: { proposal_id: johnProposalId } }, { logger: logger });
        const johnProposalStatus = yield queryContract(terra, council, { proposal: { proposal_id: johnProposalId } });
        strictEqual(johnProposalStatus.status, 'passed');
        console.log('wait for effective delay period to end');
        yield waitUntilBlockHeight(terra, johnProposalEffectiveDelayEnd);
        console.log('execute proposal');
        yield executeContract(terra, deployer, council, { execute_proposal: { proposal_id: johnProposalId } }, { logger: logger });
        console.log('verify second version of `council` contract');
        const versionResponse2 = yield queryContract(terra, council, { get_version: {} });
        strictEqual(versionResponse2.version, 'two');
    }
    console.log('OK');
    logger.showGasConsumption();
}))();
