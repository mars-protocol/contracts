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
import { queryContract } from './helpers.js';
import { LCDClient, LocalTerra } from '@terra-money/terra.js';
import { existsSync, mkdirSync, writeFileSync } from 'fs';
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        let terra;
        let redBankContractAddress = process.env.REDBANK_ADDRESS;
        if (process.env.NETWORK === 'testnet') {
            terra = new LCDClient({
                URL: 'https://bombay-lcd.terra.dev',
                chainID: 'bombay-12',
            });
        }
        else if (process.env.NETWORK === 'mainnet') {
            terra = new LCDClient({
                URL: 'https://lcd.terra.dev',
                chainID: 'columbus-5',
            });
        }
        else {
            terra = new LocalTerra();
        }
        const marketsListResult = yield queryContract(terra, redBankContractAddress, { markets_list: {} });
        const { markets_list } = marketsListResult;
        const marketInfo = {};
        for (let market of markets_list) {
            const { denom, ma_token_address } = market;
            const tokenInfoQuery = { token_info: {} };
            let { decimals } = yield queryContract(terra, ma_token_address, tokenInfoQuery);
            marketInfo[ma_token_address] = { denom, decimals };
        }
        const output = {};
        output.contracts = { redBankContractAddress };
        output.whitelist = marketInfo;
        const json = JSON.stringify(output);
        const dir = 'whitelists';
        const fileName = `${process.env.NETWORK || 'localterra'}.json`;
        if (!existsSync(dir)) {
            mkdirSync(dir);
        }
        writeFileSync(`${dir}/${fileName}`, json, { encoding: 'utf8' });
    });
}
main().catch((err) => console.log(err));
