var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { Int } from '@terra-money/terra.js';
import { strictEqual, strict as assert } from 'assert';
import { executeContract, queryContract, sleep, toEncodedBinary } from '../helpers.js';
// cw20
export function queryBalanceCw20(terra, userAddress, contractAddress) {
    return __awaiter(this, void 0, void 0, function* () {
        const result = yield queryContract(terra, contractAddress, { balance: { address: userAddress } });
        return parseInt(result.balance);
    });
}
export function mintCw20(terra, wallet, contract, recipient, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, contract, {
            mint: {
                recipient,
                amount: String(amount),
            },
        }, { logger: logger });
    });
}
export function transferCw20(terra, wallet, contract, recipient, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, contract, {
            transfer: {
                amount: String(amount),
                recipient,
            },
        }, { logger: logger });
    });
}
// terra native coins
export function queryBalanceNative(terra, address, denom) {
    return __awaiter(this, void 0, void 0, function* () {
        const [balances, _] = yield terra.bank.balance(address);
        const balance = balances.get(denom);
        if (balance === undefined) {
            return 0;
        }
        return balance.amount.toNumber();
    });
}
export function computeTax(terra, coin) {
    return __awaiter(this, void 0, void 0, function* () {
        const DECIMAL_FRACTION = new Int('1000000000000000000'); // 10^18
        const taxRate = yield terra.treasury.taxRate();
        const taxCap = (yield terra.treasury.taxCap(coin.denom)).amount;
        const amount = coin.amount;
        const tax = amount.sub(amount.mul(DECIMAL_FRACTION).div(DECIMAL_FRACTION.mul(taxRate).add(DECIMAL_FRACTION)));
        return tax.gt(taxCap) ? taxCap : tax;
    });
}
export function deductTax(terra, coin) {
    return __awaiter(this, void 0, void 0, function* () {
        return coin.amount.sub(yield computeTax(terra, coin)).floor();
    });
}
// governance
export function castVote(terra, wallet, council, proposalId, vote, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, council, {
            cast_vote: {
                proposal_id: proposalId,
                vote,
            },
        }, { logger: logger });
    });
}
// red bank
export function setAssetOraclePriceSource(terra, wallet, oracle, asset, price, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        yield executeContract(terra, wallet, oracle, {
            set_asset: {
                asset: asset,
                price_source: { fixed: { price: String(price) } },
            },
        }, { logger: logger });
    });
}
export function queryMaAssetAddress(terra, redBank, asset) {
    return __awaiter(this, void 0, void 0, function* () {
        const market = yield queryContract(terra, redBank, { market: { asset } });
        return market.ma_token_address;
    });
}
export function depositNative(terra, wallet, redBank, denom, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, redBank, { deposit_native: { denom } }, { coins: `${amount}${denom}`, logger: logger });
    });
}
export function depositCw20(terra, wallet, redBank, contract, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, contract, {
            send: {
                contract: redBank,
                amount: String(amount),
                msg: toEncodedBinary({ deposit_cw20: {} }),
            },
        }, { logger: logger });
    });
}
// TODO merge borrow functions into one
export function borrowNative(terra, wallet, redBank, denom, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, redBank, {
            borrow: {
                asset: { native: { denom: denom } },
                amount: String(amount),
            },
        }, { logger: logger });
    });
}
export function borrowCw20(terra, wallet, redBank, contract, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, redBank, {
            borrow: {
                asset: { cw20: { contract_addr: contract } },
                amount: String(amount),
            },
        }, { logger: logger });
    });
}
export function withdraw(terra, wallet, redBank, asset, amount, logger) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield executeContract(terra, wallet, redBank, {
            withdraw: {
                asset,
                amount: String(amount),
            },
        }, { logger: logger });
    });
}
// blockchain
export function getBlockHeight(terra, txResult) {
    return __awaiter(this, void 0, void 0, function* () {
        yield sleep(100);
        const txInfo = yield terra.tx.txInfo(txResult.txhash);
        return txInfo.height;
    });
}
export function getTxTimestamp(terra, result) {
    return __awaiter(this, void 0, void 0, function* () {
        const txInfo = yield terra.tx.txInfo(result.txhash);
        return Date.parse(txInfo.timestamp) / 1000; // seconds
    });
}
export function waitUntilBlockHeight(terra, blockHeight) {
    return __awaiter(this, void 0, void 0, function* () {
        const maxTries = 10;
        let tries = 0;
        let backoff = 1;
        while (true) {
            const latestBlock = yield terra.tendermint.blockInfo();
            const latestBlockHeight = parseInt(latestBlock.block.header.height);
            if (latestBlockHeight >= blockHeight) {
                break;
            }
            // timeout
            tries++;
            if (tries == maxTries) {
                throw new Error(`timed out waiting for block height ${blockHeight}, current block height: ${latestBlockHeight}`);
            }
            // exponential backoff
            yield sleep(backoff * 1000);
            backoff *= 2;
        }
    });
}
// testing
export function approximateEqual(actual, expected, tol) {
    try {
        assert(actual >= expected - tol && actual <= expected + tol);
    }
    catch (error) {
        strictEqual(actual, expected);
    }
}
