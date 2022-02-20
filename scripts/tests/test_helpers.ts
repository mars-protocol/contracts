import {
  BlockTxBroadcastResult,
  Coin,
  Int,
  LCDClient,
  Wallet
} from "@terra-money/terra.js"
import {
  strictEqual,
  strict as assert
} from "assert"
import {
  executeContract, Logger,
  queryContract,
  sleep,
  toEncodedBinary
} from "../helpers.js"

// assets

interface Native { native: { denom: string } }

interface CW20 { cw20: { contract_addr: string } }

export type Asset = Native | CW20

// cw20

export async function queryBalanceCw20(
  terra: LCDClient,
  userAddress: string,
  contractAddress: string,
) {
  const result = await queryContract(terra, contractAddress, { balance: { address: userAddress } })
  return parseInt(result.balance)
}

export async function mintCw20(
  terra: LCDClient,
  wallet: Wallet,
  contract: string,
  recipient: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, contract,
    {
      mint: {
        recipient,
        amount: String(amount)
      }
    },
    { logger: logger }
  )
}

export async function transferCw20(
  terra: LCDClient,
  wallet: Wallet,
  contract: string,
  recipient: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, contract,
    {
      transfer: {
        amount: String(amount),
        recipient
      }
    },
    { logger: logger }
  )
}

// terra native coins

export async function queryBalanceNative(
  terra: LCDClient,
  address: string,
  denom: string,
) {
  const [balances, _] = await terra.bank.balance(address)
  const balance = balances.get(denom)
  if (balance === undefined) {
    return 0
  }
  return balance.amount.toNumber()
}

export async function computeTax(
  terra: LCDClient,
  coin: Coin,
) {
  const DECIMAL_FRACTION = new Int("1000000000000000000") // 10^18
  const taxRate = await terra.treasury.taxRate()
  const taxCap = (await terra.treasury.taxCap(coin.denom)).amount
  const amount = coin.amount
  const tax = amount.sub(
    amount
      .mul(DECIMAL_FRACTION)
      .div(DECIMAL_FRACTION.mul(taxRate).add(DECIMAL_FRACTION))
  )
  return tax.gt(taxCap) ? taxCap : tax
}

export async function deductTax(
  terra: LCDClient,
  coin: Coin,
) {
  return coin.amount.sub(await computeTax(terra, coin)).floor()
}

// governance

export async function castVote(
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

// red bank

export async function setAssetOraclePriceSource(
  terra: LCDClient,
  wallet: Wallet,
  oracle: string,
  asset: Asset,
  price: number,
  logger?: Logger
) {
  await executeContract(terra, wallet, oracle,
    {
      set_asset: {
        asset: asset,
        price_source: { fixed: { price: String(price) } }
      }
    },
    { logger: logger }
  )
}

export async function queryMaAssetAddress(
  terra: LCDClient,
  redBank: string,
  asset: Asset,
): Promise<string> {
  const market = await queryContract(terra, redBank, { market: { asset } })
  return market.ma_token_address
}

export async function depositNative(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  denom: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, redBank,
    { deposit_native: { denom } },
    { coins: `${amount}${denom}`, logger: logger }
  )
}

export async function depositCw20(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  contract: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, contract,
    {
      send: {
        contract: redBank,
        amount: String(amount),
        msg: toEncodedBinary({ deposit_cw20: {} })
      }
    },
    { logger: logger }
  )
}

// TODO merge borrow functions into one
export async function borrowNative(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  denom: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, redBank,
    {
      borrow: {
        asset: { native: { denom: denom } },
        amount: String(amount)
      }
    },
    { logger: logger }
  )
}

export async function borrowCw20(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  contract: string,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, redBank,
    {
      borrow: {
        asset: { cw20: { contract_addr: contract } },
        amount: String(amount)
      }
    },
    { logger: logger }
  )
}

export async function withdraw(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  asset: Asset,
  amount: number,
  logger?: Logger
) {
  return await executeContract(terra, wallet, redBank,
    {
      withdraw: {
        asset,
        amount: String(amount),
      }
    },
    { logger: logger }
  )
}

// blockchain

export async function getBlockHeight(
  terra: LCDClient,
  txResult: BlockTxBroadcastResult,
) {
  await sleep(100)
  const txInfo = await terra.tx.txInfo(txResult.txhash)
  return txInfo.height
}

export async function getTxTimestamp(
  terra: LCDClient,
  result: BlockTxBroadcastResult,
) {
  const txInfo = await terra.tx.txInfo(result.txhash)
  return Date.parse(txInfo.timestamp) / 1000 // seconds
}

export async function waitUntilBlockHeight(
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

// testing

export function approximateEqual(
  actual: number,
  expected: number,
  tol: number,
) {
  try {
    assert(actual >= expected - tol && actual <= expected + tol)
  } catch (error) {
    strictEqual(actual, expected)
  }
}
