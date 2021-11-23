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
  executeContract,
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
) {
  return await executeContract(terra, wallet, contract,
    {
      mint: {
        recipient,
        amount: String(amount)
      }
    }
  )
}

export async function transferCw20(
  terra: LCDClient,
  wallet: Wallet,
  contract: string,
  recipient: string,
  amount: number,
) {
  return await executeContract(terra, wallet, contract,
    {
      transfer: {
        amount: String(amount),
        recipient
      }
    }
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

// red bank

export async function setAssetOraclePriceSource(
  terra: LCDClient,
  wallet: Wallet,
  oracle: string,
  asset: Asset,
  price: number,
) {
  await executeContract(terra, wallet, oracle,
    {
      set_asset: {
        asset: asset,
        price_source: { fixed: { price: String(price) } }
      }
    }
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
) {
  return await executeContract(terra, wallet, redBank,
    { deposit_native: { denom } },
    `${amount}${denom}`
  )
}

export async function depositCw20(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  contract: string,
  amount: number,
) {
  return await executeContract(terra, wallet, contract,
    {
      send: {
        contract: redBank,
        amount: String(amount),
        msg: toEncodedBinary({ deposit_cw20: {} })
      }
    }
  )
}

// TODO merge borrow functions into one
export async function borrowNative(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  denom: string,
  amount: number,
) {
  return await executeContract(terra, wallet, redBank,
    {
      borrow: {
        asset: { native: { denom: denom } },
        amount: String(amount)
      }
    }
  )
}

export async function borrowCw20(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  contract: string,
  amount: number,
) {
  return await executeContract(terra, wallet, redBank,
    {
      borrow: {
        asset: { cw20: { contract_addr: contract } },
        amount: String(amount)
      }
    }
  )
}

export async function withdraw(
  terra: LCDClient,
  wallet: Wallet,
  redBank: string,
  asset: Asset,
  amount: number,
) {
  return await executeContract(terra, wallet, redBank,
    {
      withdraw: {
        asset,
        amount: String(amount),
      }
    }
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
