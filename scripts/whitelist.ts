import 'dotenv/config.js';
import { queryContract } from "./helpers.js";
import { LCDClient, LocalTerra } from "@terra-money/terra.js";
import { existsSync, mkdirSync, writeFileSync } from 'fs';

async function main() {
  let terra;
  let redBankContractAddress = process.env.REDBANK_ADDRESS!;

  if (process.env.NETWORK === "testnet") {
    terra = new LCDClient({
      URL: 'https://bombay-lcd.terra.dev',
      chainID: 'bombay-12'
    })
  } else if (process.env.NETWORK === "mainnet") {
    terra = new LCDClient({
      URL: 'https://lcd.terra.dev',
      chainID: 'columbus-5'
    })
  } else {
    terra = new LocalTerra();
  }

  const marketsListResult = await queryContract(terra, redBankContractAddress, { "markets_list": {} });
  const { markets_list } = marketsListResult;
  const marketInfo: any = {};

  for (let market of markets_list) {
    const { denom, ma_token_address } = market;
    const tokenInfoQuery = { "token_info": {} };
    let { decimals } = await queryContract(terra, ma_token_address, tokenInfoQuery);
    marketInfo[ma_token_address] = { denom, decimals }
  }

  const output: any = {};
  output.contracts = { redBankContractAddress };
  output.whitelist = marketInfo;

  const json = JSON.stringify(output);

  const dir = "whitelists"
  const fileName = `${process.env.NETWORK || 'localterra'}.json`
  if (!existsSync(dir)) {
    mkdirSync(dir);
  }
  writeFileSync(`${dir}/${fileName}`, json, { 'encoding': 'utf8' });
}

main().catch(err => console.log(err));
