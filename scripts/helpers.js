var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { Coin, isTxError, MnemonicKey, MsgExecuteContract, MsgInstantiateContract, MsgMigrateContract, MsgUpdateContractAdmin, MsgStoreCode, } from '@terra-money/terra.js';
import { readFileSync } from 'fs';
import { CustomError } from 'ts-custom-error';
// LCD endpoints are load balanced, so txs can't be sent too fast, otherwise account sequence queries
// may resolve an older state depending on which lcd you end up with. Generally 1000 ms is is enough
// for all nodes to sync up.
let TIMEOUT = 1000;
export function setTimeoutDuration(t) {
    TIMEOUT = t;
}
export function getTimeoutDuration() {
    return TIMEOUT;
}
let GAS_ADJUSTMENT = 1.2;
export function setGasAdjustment(g) {
    GAS_ADJUSTMENT = g;
}
export function getGasAdjustment() {
    return GAS_ADJUSTMENT;
}
export function sleep(timeout) {
    return __awaiter(this, void 0, void 0, function* () {
        yield new Promise((resolve) => setTimeout(resolve, timeout));
    });
}
export class TransactionError extends CustomError {
    constructor(code, codespace, rawLog) {
        super('transaction failed');
        this.code = code;
        this.codespace = codespace;
        this.rawLog = rawLog;
    }
}
export class Logger {
    constructor(logGasConsumption = true) {
        this.logGasConsumption = logGasConsumption;
        this.gasConsumptions = [];
    }
    addGasConsumption(msg, gasUsed) {
        const msgStr = JSON.stringify(msg);
        this.gasConsumptions.push({ msg: msgStr, gasUsed: gasUsed });
    }
    showGasConsumption() {
        if (this.gasConsumptions.length == 0) {
            return;
        }
        this.gasConsumptions.sort((a, b) => b.gasUsed - a.gasUsed);
        console.log('--- MAX GAS CONSUMPTION ---');
        const maxGasConsumption = this.gasConsumptions[0];
        console.log('gas used: ', maxGasConsumption.gasUsed, ', msg: ', maxGasConsumption.msg);
        console.log('--- AVERAGE GAS CONSUMPTION ---');
        const sumOfGasConsumption = this.gasConsumptions.reduce((a, b) => a + b.gasUsed, 0);
        const avgOfGasConsumption = sumOfGasConsumption / this.gasConsumptions.length;
        console.log('avg gas used: ', avgOfGasConsumption);
        console.log('--- SORTED GAS CONSUMPTION (DESCENDING) ---');
        this.gasConsumptions.forEach(function ({ msg, gasUsed }) {
            console.log('gas used: ', gasUsed, ', msg: ', msg);
        });
    }
}
export function createTransaction(wallet, msg) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield wallet.createTx({
            msgs: [msg],
            gasAdjustment: GAS_ADJUSTMENT,
        });
    });
}
export function broadcastTransaction(terra, signedTx) {
    return __awaiter(this, void 0, void 0, function* () {
        const result = yield terra.tx.broadcast(signedTx);
        yield sleep(TIMEOUT);
        return result;
    });
}
export function performTransaction(terra, wallet, msg) {
    return __awaiter(this, void 0, void 0, function* () {
        const tx = yield createTransaction(wallet, msg);
        const { account_number, sequence } = yield wallet.accountNumberAndSequence();
        const signedTx = yield wallet.key.signTx(tx, {
            accountNumber: account_number,
            sequence: sequence,
            chainID: terra.config.chainID,
            signMode: 1, // SignMode.SIGN_MODE_DIRECT
        });
        const result = yield broadcastTransaction(terra, signedTx);
        if (isTxError(result)) {
            throw transactionErrorFromResult(result);
        }
        return result;
    });
}
export function transactionErrorFromResult(result) {
    return new TransactionError(result.code, result.codespace, result.raw_log);
}
export function uploadContract(terra, wallet, filepath) {
    return __awaiter(this, void 0, void 0, function* () {
        const contract = readFileSync(filepath, 'base64');
        const uploadMsg = new MsgStoreCode(wallet.key.accAddress, contract);
        let result = yield performTransaction(terra, wallet, uploadMsg);
        return Number(result.logs[0].eventsByType.store_code.code_id[0]); // code_id
    });
}
export function instantiateContract(terra, wallet, codeId, msg, opts = {}) {
    return __awaiter(this, void 0, void 0, function* () {
        let admin = opts.admin;
        if (admin == undefined) {
            admin = wallet.key.accAddress;
        }
        const instantiateMsg = new MsgInstantiateContract(wallet.key.accAddress, admin, codeId, msg, undefined);
        let result = yield performTransaction(terra, wallet, instantiateMsg);
        const attributes = result.logs[0].events[0].attributes;
        return attributes[attributes.length - 1].value; // contract address
    });
}
export function executeContract(terra, wallet, contractAddress, msg, opts = {}) {
    return __awaiter(this, void 0, void 0, function* () {
        const coins = opts.coins;
        const logger = opts.logger;
        const executeMsg = new MsgExecuteContract(wallet.key.accAddress, contractAddress, msg, coins);
        const result = yield performTransaction(terra, wallet, executeMsg);
        if (logger !== undefined && logger.logGasConsumption) {
            // save gas consumption during contract execution
            logger.addGasConsumption(msg, result.gas_used);
        }
        return result;
    });
}
export function queryContract(terra, contractAddress, query) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield terra.wasm.contractQuery(contractAddress, query);
    });
}
export function deployContract(terra, wallet, filepath, initMsg) {
    return __awaiter(this, void 0, void 0, function* () {
        const codeId = yield uploadContract(terra, wallet, filepath);
        return yield instantiateContract(terra, wallet, codeId, initMsg);
    });
}
export function updateContractAdmin(terra, admin, newAdmin, contractAddress) {
    return __awaiter(this, void 0, void 0, function* () {
        const updateContractAdminMsg = new MsgUpdateContractAdmin(admin.key.accAddress, newAdmin, contractAddress);
        return yield performTransaction(terra, admin, updateContractAdminMsg);
    });
}
export function migrate(terra, wallet, contractAddress, newCodeId) {
    return __awaiter(this, void 0, void 0, function* () {
        const migrateMsg = new MsgMigrateContract(wallet.key.accAddress, contractAddress, newCodeId, {});
        return yield performTransaction(terra, wallet, migrateMsg);
    });
}
export function recover(terra, mnemonic) {
    const mk = new MnemonicKey({ mnemonic: mnemonic });
    return terra.wallet(mk);
}
export function initialize(terra) {
    const mk = new MnemonicKey();
    console.log(`Account Address: ${mk.accAddress}`);
    console.log(`MnemonicKey: ${mk.mnemonic}`);
    return terra.wallet(mk);
}
export function setupOracle(terra, wallet, contractAddress, initialAssets, oracleFactoryAddress, isTestnet) {
    var _a, _b;
    return __awaiter(this, void 0, void 0, function* () {
        console.log('Setting up oracle assets...');
        for (let asset of initialAssets) {
            console.log(`Setting price source for ${asset.denom || asset.symbol || asset.contract_addr}`);
            let assetType;
            let assetPriceSource;
            if (asset.denom) {
                assetType = {
                    native: {
                        denom: asset.denom,
                    },
                };
                assetPriceSource = assetType;
            }
            else if (asset.contract_addr) {
                assetType = {
                    cw20: {
                        contract_addr: asset.contract_addr,
                    },
                };
                const pairQueryMsg = {
                    pair: {
                        asset_infos: [
                            {
                                token: {
                                    contract_addr: asset.contract_addr,
                                },
                            },
                            {
                                native_token: {
                                    denom: 'uusd',
                                },
                            },
                        ],
                    },
                };
                let pairQueryResponse;
                try {
                    pairQueryResponse = yield queryContract(terra, oracleFactoryAddress, pairQueryMsg);
                }
                catch (error) {
                    if (error.response.data.message.includes('PairInfo not found')) {
                        console.log('Pair not found, creating pair...');
                        const createPairMsg = {
                            create_pair: {
                                pair_type: {
                                    xyk: {},
                                },
                                asset_infos: [
                                    {
                                        token: {
                                            contract_addr: asset.contract_addr,
                                        },
                                    },
                                    {
                                        native_token: {
                                            denom: 'uusd',
                                        },
                                    },
                                ],
                            },
                        };
                        yield executeContract(terra, wallet, oracleFactoryAddress, createPairMsg);
                        console.log('Pair created');
                        pairQueryResponse = yield queryContract(terra, oracleFactoryAddress, pairQueryMsg);
                    }
                    else {
                        console.log(((_b = (_a = error.response) === null || _a === void 0 ? void 0 : _a.data) === null || _b === void 0 ? void 0 : _b.error) || 'Error: pair contract query failed');
                        continue;
                    }
                }
                if (!pairQueryResponse.contract_addr) {
                    console.log('Error: something bad happened while trying to get oracle pairs contract address');
                }
                assetPriceSource = {
                    astroport_spot: {
                        pair_address: pairQueryResponse.contract_addr,
                    },
                };
            }
            else {
                console.log(`INVALID ASSET: no denom or contract_addr`);
                return;
            }
            let setAssetMsg = {
                set_asset: {
                    asset: assetType,
                    price_source: assetPriceSource,
                },
            };
            yield executeContract(terra, wallet, contractAddress, setAssetMsg);
            console.log(`Set ${asset.denom || asset.symbol || asset.contract_addr}`);
        }
    });
}
export function setupRedBank(terra, wallet, contractAddress, options) {
    var _a, _b, _c;
    return __awaiter(this, void 0, void 0, function* () {
        console.log('Setting up initial asset liquidity pools...');
        const initialAssets = (_a = options.initialAssets) !== null && _a !== void 0 ? _a : [];
        const initialDeposits = (_b = options.initialDeposits) !== null && _b !== void 0 ? _b : [];
        const initialBorrows = (_c = options.initialBorrows) !== null && _c !== void 0 ? _c : [];
        for (let asset of initialAssets) {
            console.log(`Initializing ${asset.denom || asset.symbol || asset.contract_addr}`);
            let assetType = asset.denom
                ? {
                    native: {
                        denom: asset.denom,
                    },
                }
                : asset.contract_addr
                    ? {
                        cw20: {
                            contract_addr: asset.contract_addr,
                        },
                    }
                    : undefined;
            let initAssetMsg = {
                init_asset: {
                    asset: assetType,
                    asset_params: asset.init_params,
                    asset_symbol: asset.symbol,
                },
            };
            yield executeContract(terra, wallet, contractAddress, initAssetMsg);
            console.log(`Initialized ${asset.denom || asset.symbol || asset.contract_addr}`);
        }
        for (let deposit of initialDeposits) {
            const { account, assets } = deposit;
            console.log(`### Deposits for account ${account.key.accAddress}: `);
            for (const asset of Object.keys(assets)) {
                const amount = assets[asset];
                const coins = new Coin(asset, amount);
                const depositMsg = { deposit_native: { denom: asset } };
                const executeDepositMsg = new MsgExecuteContract(account.key.accAddress, contractAddress, depositMsg, [coins]);
                yield performTransaction(terra, account, executeDepositMsg);
                console.log(`Deposited ${amount} ${asset}`);
            }
        }
        for (let borrow of initialBorrows) {
            const { account, assets } = borrow;
            console.log(`### Borrows for account ${account.key.accAddress}: `);
            for (const asset of Object.keys(assets)) {
                const amount = assets[asset];
                const borrowMsg = {
                    borrow: {
                        asset: {
                            native: {
                                denom: asset,
                            },
                        },
                        amount: amount.toString(),
                    },
                };
                const executeBorrowMsg = new MsgExecuteContract(account.key.accAddress, contractAddress, borrowMsg);
                yield performTransaction(terra, account, executeBorrowMsg);
                console.log(`Borrowed ${amount} ${asset}`);
            }
        }
    });
}
export function toEncodedBinary(object) {
    return Buffer.from(JSON.stringify(object)).toString('base64');
}
