import {
  BlockTxBroadcastResult,
  Coin,
  isTxError,
  LCDClient,
  MnemonicKey,
  Msg,
  MsgExecuteContract,
  MsgInstantiateContract,
  MsgMigrateContract,
  MsgUpdateContractAdmin,
  MsgStoreCode,
  Tx,
  TxError,
  Wallet
} from '@terra-money/terra.js';
import { readFileSync } from 'fs';
import { CustomError } from 'ts-custom-error'

// LCD endpoints are load balanced, so txs can't be sent too fast, otherwise account sequence queries
// may resolve an older state depending on which lcd you end up with. Generally 1000 ms is is enough
// for all nodes to sync up.
let TIMEOUT = 1000

export function setTimeoutDuration(t: number) {
  TIMEOUT = t
}

export function getTimeoutDuration() {
  return TIMEOUT
}

let GAS_ADJUSTMENT = 1.2

export function setGasAdjustment(g: number) {
  GAS_ADJUSTMENT = g
}

export function getGasAdjustment() {
  return GAS_ADJUSTMENT
}

export async function sleep(timeout: number) {
  await new Promise(resolve => setTimeout(resolve, timeout))
}

export class TransactionError extends CustomError {
  public constructor(
    public code: number | string,
    public codespace: string | undefined,
    public rawLog: string,
  ) {
    super("transaction failed")
  }
}

interface Opts {
  admin?: string,
  coins?: string,
  logger?: Logger
}

export class Logger {
  private gasConsumptions: Array<{msg: string, gasUsed: number}> = []

  constructor(readonly logGasConsumption: boolean = true) {}

  addGasConsumption(msg: object, gasUsed: number) {
    const msgStr = JSON.stringify(msg)
    this.gasConsumptions.push({msg: msgStr, gasUsed: gasUsed})
  }

  showGasConsumption() {
    if (this.gasConsumptions.length == 0) {
      return;
    }

    this.gasConsumptions.sort((a, b) => b.gasUsed - a.gasUsed);

    console.log("--- MAX GAS CONSUMPTION ---")
    const maxGasConsumption = this.gasConsumptions[0]
    console.log("gas used: ", maxGasConsumption.gasUsed, ", msg: ", maxGasConsumption.msg)

    console.log("--- AVERAGE GAS CONSUMPTION ---")
    const sumOfGasConsumption = this.gasConsumptions.reduce((a, b) => a + b.gasUsed, 0);
    const avgOfGasConsumption = sumOfGasConsumption / this.gasConsumptions.length
    console.log("avg gas used: ", avgOfGasConsumption)

    console.log("--- SORTED GAS CONSUMPTION (DESCENDING) ---")
    this.gasConsumptions.forEach(function ({msg, gasUsed}) {
      console.log("gas used: ", gasUsed, ", msg: ", msg)
    })
  }
}

export async function createTransaction(wallet: Wallet, msg: Msg) {
  return await wallet.createTx({
    msgs: [msg],
    gasAdjustment: GAS_ADJUSTMENT,
  })
}

export async function broadcastTransaction(terra: LCDClient, signedTx: Tx) {
  const result = await terra.tx.broadcast(signedTx)
  await sleep(TIMEOUT)
  return result
}

export async function performTransaction(terra: LCDClient, wallet: Wallet, msg: Msg) {
  const tx = await createTransaction(wallet, msg)
  const { account_number, sequence } = await wallet.accountNumberAndSequence()
  const signedTx = await wallet.key.signTx(tx,
    {
      accountNumber: account_number,
      sequence: sequence,
      chainID: terra.config.chainID,
      signMode: 1, // SignMode.SIGN_MODE_DIRECT
    }
  )
  const result = await broadcastTransaction(terra, signedTx)
  if (isTxError(result)) {
    throw transactionErrorFromResult(result)
  }
  return result
}

export function transactionErrorFromResult(result: BlockTxBroadcastResult & TxError) {
  return new TransactionError(result.code, result.codespace, result.raw_log)
}

export async function uploadContract(terra: LCDClient, wallet: Wallet, filepath: string) {
  const contract = readFileSync(filepath, 'base64');
  const uploadMsg = new MsgStoreCode(wallet.key.accAddress, contract);
  let result = await performTransaction(terra, wallet, uploadMsg);
  return Number(result.logs[0].eventsByType.store_code.code_id[0]) // code_id
}

export async function instantiateContract(terra: LCDClient, wallet: Wallet, codeId: number, msg: object, opts: Opts = {}) {
  let admin = opts.admin
  if (admin == undefined) {
    admin = wallet.key.accAddress
  }
  const instantiateMsg = new MsgInstantiateContract(wallet.key.accAddress, admin, codeId, msg, undefined);
  let result = await performTransaction(terra, wallet, instantiateMsg)
  const attributes = result.logs[0].events[0].attributes
  return attributes[attributes.length - 1].value // contract address
}

export async function executeContract(terra: LCDClient, wallet: Wallet, contractAddress: string, msg: object, opts: Opts = {}) {
  const coins = opts.coins
  const logger = opts.logger

  const executeMsg = new MsgExecuteContract(wallet.key.accAddress, contractAddress, msg, coins);
  const result = await performTransaction(terra, wallet, executeMsg);

  if (logger !== undefined && logger.logGasConsumption) {
    // save gas consumption during contract execution
    logger.addGasConsumption(msg, result.gas_used)
  }

  return result;
}

export async function queryContract(terra: LCDClient, contractAddress: string, query: object): Promise<any> {
  return await terra.wasm.contractQuery(contractAddress, query)
}

export async function deployContract(terra: LCDClient, wallet: Wallet, filepath: string, initMsg: object) {
  const codeId = await uploadContract(terra, wallet, filepath);
  return await instantiateContract(terra, wallet, codeId, initMsg);
}

export async function updateContractAdmin(terra: LCDClient, admin: Wallet, newAdmin: string, contractAddress: string) {
  const updateContractAdminMsg = new MsgUpdateContractAdmin(admin.key.accAddress, newAdmin, contractAddress);
  return await performTransaction(terra, admin, updateContractAdminMsg);
}

export async function migrate(terra: LCDClient, wallet: Wallet, contractAddress: string, newCodeId: number) {
  const migrateMsg = new MsgMigrateContract(wallet.key.accAddress, contractAddress, newCodeId, {});
  return await performTransaction(terra, wallet, migrateMsg);
}

export function recover(terra: LCDClient, mnemonic: string) {
  const mk = new MnemonicKey({ mnemonic: mnemonic });
  return terra.wallet(mk);
}

export function initialize(terra: LCDClient) {
  const mk = new MnemonicKey();

  console.log(`Account Address: ${mk.accAddress}`);
  console.log(`MnemonicKey: ${mk.mnemonic}`);

  return terra.wallet(mk);
}

export async function setupOracle(
  terra: LCDClient, wallet: Wallet, contractAddress: string, initialAssets: Asset[], oracleFactoryAddress: string, isTestnet: boolean
) {
  console.log("Setting up oracle assets...");

  for (let asset of initialAssets) {
    console.log(`Setting price source for ${asset.denom || asset.symbol || asset.contract_addr}`);

    let assetType
    let assetPriceSource

    if (asset.denom) {
      assetType = {
        "native": {
          "denom": asset.denom,
        }
      }
      assetPriceSource = assetType
    } else if (asset.contract_addr) {
      assetType = {
        "cw20": {
          "contract_addr": asset.contract_addr,
        }
      }

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
                denom: "uusd",
              },
            },
          ],
        },
      }

      let pairQueryResponse
      try {
        pairQueryResponse = await queryContract(terra, oracleFactoryAddress, pairQueryMsg)
      } catch (error: any) {
        if (error.response.data.message.includes("PairInfo not found")) {
          console.log("Pair not found, creating pair...");

          const createPairMsg = {
            "create_pair": {
              "pair_type": {
                "xyk": {}
              },
              "asset_infos": [
                {
                  "token": {
                    "contract_addr": asset.contract_addr
                  }
                },
                {
                  "native_token": {
                    "denom": "uusd"
                  }
                }
              ]
            }
          }

          await executeContract(terra, wallet, oracleFactoryAddress, createPairMsg);
          console.log("Pair created");

          pairQueryResponse = await queryContract(terra, oracleFactoryAddress, pairQueryMsg)
        } else {
          console.log(error.response?.data?.error || "Error: pair contract query failed")
          continue
        }
      }

      if (!pairQueryResponse.contract_addr) {
        console.log("Error: something bad happened while trying to get oracle pairs contract address")
      }

      assetPriceSource = {
        "astroport_spot": {
          "pair_address": pairQueryResponse.contract_addr
        }
      }
    } else {
      console.log(`INVALID ASSET: no denom or contract_addr`);
      return
    }

    let setAssetMsg = {
      "set_asset": {
        "asset": assetType,
        "price_source": assetPriceSource,
      },
    };

    await executeContract(terra, wallet, contractAddress, setAssetMsg);
    console.log(`Set ${asset.denom || asset.symbol || asset.contract_addr}`);
  }
}

export async function setupRedBank(terra: LCDClient, wallet: Wallet, contractAddress: string, options: any) {
  console.log("Setting up initial asset liquidity pools...");

  const initialAssets = options.initialAssets ?? [];
  const initialDeposits = options.initialDeposits ?? [];
  const initialBorrows = options.initialBorrows ?? [];

  for (let asset of initialAssets) {
    console.log(`Initializing ${asset.denom || asset.symbol || asset.contract_addr}`);

    let assetType = asset.denom
      ? {
        "native": {
          "denom": asset.denom,
        }
      }
      : asset.contract_addr
        ? {
          "cw20": {
            "contract_addr": asset.contract_addr,
          }
        }
        : undefined

    let initAssetMsg = {
      "init_asset": {
        "asset": assetType,
        "asset_params": asset.init_params,
        "asset_symbol": asset.symbol,
      },
    };

    await executeContract(terra, wallet, contractAddress, initAssetMsg);
    console.log(`Initialized ${asset.denom || asset.symbol || asset.contract_addr}`);
  }

  for (let deposit of initialDeposits) {
    const { account, assets } = deposit;
    console.log(`### Deposits for account ${account.key.accAddress}: `);
    for (const asset of Object.keys(assets)) {
      const amount = assets[asset]
      const coins = new Coin(asset, amount);
      const depositMsg = { "deposit_native": { "denom": asset } };
      const executeDepositMsg = new MsgExecuteContract(account.key.accAddress, contractAddress, depositMsg, [coins]);
      await performTransaction(terra, account, executeDepositMsg);
      console.log(`Deposited ${amount} ${asset}`);
    }
  }

  for (let borrow of initialBorrows) {
    const { account, assets } = borrow;
    console.log(`### Borrows for account ${account.key.accAddress}: `);
    for (const asset of Object.keys(assets)) {
      const amount = assets[asset]
      const borrowMsg = {
        "borrow": {
          "asset": {
            "native": {
              "denom": asset
            }
          },
          "amount": amount.toString()
        }
      };
      const executeBorrowMsg = new MsgExecuteContract(account.key.accAddress, contractAddress, borrowMsg);
      await performTransaction(terra, account, executeBorrowMsg);
      console.log(`Borrowed ${amount} ${asset}`);
    }
  }
}

export function toEncodedBinary(object: any) {
  return Buffer.from(JSON.stringify(object)).toString('base64');
}
