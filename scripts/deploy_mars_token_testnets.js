var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import 'dotenv/config.js';
import { deployContract, queryContract, recover, setTimeoutDuration } from './helpers.js';
import { LCDClient, LocalTerra } from '@terra-money/terra.js';
import { join } from 'path';
// consts
const MARS_ARTIFACTS_PATH = '../artifacts';
// main
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        let terra;
        let wallet;
        const isTestnet = process.env.NETWORK === 'testnet';
        if (process.env.NETWORK === 'testnet') {
            terra = new LCDClient({
                URL: 'https://bombay-lcd.terra.dev',
                chainID: 'bombay-12',
            });
            wallet = recover(terra, process.env.TEST_MAIN);
        }
        else {
            terra = new LocalTerra();
            wallet = terra.wallets.test1;
            setTimeoutDuration(0);
        }
        console.log(`Wallet address from seed: ${wallet.key.accAddress}`);
        /************************************* Deploy Minter Proxy Contract *************************************/
        console.log('Deploying Minter Proxy...');
        const minterProxyContractAddress = yield deployContract(terra, wallet, join(MARS_ARTIFACTS_PATH, 'cw1_whitelist.wasm'), {
            admins: [wallet.key.accAddress],
            mutable: true,
        });
        console.log('Minter Proxy Contract Address: ' + minterProxyContractAddress);
        /************************************* Deploy Mars token Contract *************************************/
        console.log('Deploying Mars token...');
        const marsTokenContractAddress = yield deployContract(terra, wallet, join(MARS_ARTIFACTS_PATH, 'cw20_base.wasm'), {
            name: 'Mars',
            symbol: 'MARS',
            decimals: 6,
            initial_balances: isTestnet
                ? [
                    {
                        address: wallet.key.accAddress,
                        amount: '1000000000000',
                    },
                ]
                : [],
            mint: {
                minter: minterProxyContractAddress,
            },
        });
        console.log('Mars Token Contract Address: ' + marsTokenContractAddress);
        const balanceResponse = yield queryContract(terra, marsTokenContractAddress, {
            balance: {
                address: wallet.key.accAddress,
            },
        });
        console.log(`Balance of adress ${wallet.key.accAddress}: ${balanceResponse.balance / 1e6} MARS`);
    });
}
main().catch(console.log);
