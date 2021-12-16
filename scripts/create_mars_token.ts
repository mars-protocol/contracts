/*
Script to deploy a cw20 token to Terra Columbus-5, setting the token minter to a cw1 whitelist
contract that has a multisig as the sole admin.

Dependencies:
  - cw-plus v0.9.1
  - LocalTerra (optional)
  - Set environment variables in a .env file (see below for details of the required variables)
*/

import {
  LCDClient,
  LegacyAminoMultisigPublicKey,
  LocalTerra,
  SimplePublicKey,
  Wallet
} from "@terra-money/terra.js"
import 'dotenv/config.js'
import { join } from "path"
import {
  instantiateContract,
  queryContract,
  recover,
  setTimeoutDuration,
  uploadContract
} from "./helpers.js"

// CONSTS

// Required environment variables:
const CW_PLUS_ARTIFACTS_PATH = process.env.CW_PLUS_ARTIFACTS_PATH!

// Multisig details:
const MULTISIG_PUBLIC_KEYS = (process.env.MULTISIG_PUBLIC_KEYS!)
  .split(",")
  // terrad sorts keys of multisigs by comparing bytes of their address
  .sort((a, b) => {
    return Buffer.from(
      new SimplePublicKey(a).rawAddress()
    ).compare(
      Buffer.from(
        new SimplePublicKey(b).rawAddress()
      )
    )
  })
  .map(x => new SimplePublicKey(x))
const MULTISIG_THRESHOLD = parseInt(process.env.MULTISIG_THRESHOLD!)

// For networks other than LocalTerra:
const CHAIN_ID = process.env.CHAIN_ID
const LCD_CLIENT_URL = process.env.LCD_CLIENT_URL

// Token info
const TOKEN_NAME = "Mars"
const TOKEN_SYMBOL = "MARS"
const TOKEN_DECIMALS = 6
const TOKEN_DESCRIPTION = "Mars is a fully automated, on-chain credit protocol built on Terra " +
  "and governed by a decentralised community of users and developers"
const TOKEN_PROJECT = "Mars Protocol"
const TOKEN_LOGO = "https://marsprotocol.io/mars_logo_colored.svg";

// MAIN

(async () => {
  const isLocalTerra = CHAIN_ID == "localterra" || CHAIN_ID == undefined

  let terra: LCDClient
  let wallet: Wallet

  if (isLocalTerra) {
    setTimeoutDuration(0)

    terra = new LocalTerra()

    wallet = (terra as LocalTerra).wallets.test1
  } else {
    terra = new LCDClient({
      URL: LCD_CLIENT_URL!,
      chainID: CHAIN_ID!
    })

    wallet = recover(terra, process.env.WALLET!)
    console.log("wallet:", wallet.key.accAddress)
  }

  // Multisig
  const multisigPublicKey = new LegacyAminoMultisigPublicKey(MULTISIG_THRESHOLD, MULTISIG_PUBLIC_KEYS)
  const multisigAddress = multisigPublicKey.address()
  console.log("multisig:", multisigAddress)

  // Instantiate the token minter proxy contract
  const cw1WhitelistCodeId = await uploadContract(
    terra,
    wallet,
    join(CW_PLUS_ARTIFACTS_PATH, "cw1_whitelist.wasm")
  )

  console.log("cw1 whitelist code ID:", cw1WhitelistCodeId)

  const proxyAddress = await instantiateContract(
    terra,
    wallet,
    cw1WhitelistCodeId,
    {
      mutable: true,
      admins: [multisigAddress]
    },
    { admin: multisigAddress }
  )

  console.log("proxy:", proxyAddress)
  console.log(await terra.wasm.contractInfo(proxyAddress))
  console.log(
    await queryContract(
      terra,
      proxyAddress,
      { admin_list: {} }
    )
  )

  // Instantiate Mars token contract
  const cw20CodeId = await uploadContract(
    terra,
    wallet,
    join(CW_PLUS_ARTIFACTS_PATH, "cw20_base.wasm")
  )

  console.log("cw20 code ID:", cw20CodeId)

  const marsAddress = await instantiateContract(
    terra,
    wallet,
    cw20CodeId,
    {
      name: TOKEN_NAME,
      symbol: TOKEN_SYMBOL,
      decimals: TOKEN_DECIMALS,
      initial_balances: [],
      mint: { minter: proxyAddress },
      marketing: {
        marketing: multisigAddress,
        description: TOKEN_DESCRIPTION,
        project: TOKEN_PROJECT,
        logo: { url: TOKEN_LOGO }
      }
    },
    { admin: multisigAddress }
  )

  console.log("mars:", marsAddress)
  console.log(await terra.wasm.contractInfo(marsAddress))
  console.log(
    await queryContract(
      terra,
      marsAddress,
      { token_info: {} }
    )
  )
  console.log(
    await queryContract(
      terra,
      marsAddress,
      { minter: {} }
    )
  )
  console.log(
    await queryContract(
      terra,
      marsAddress,
      { marketing_info: {} }
    )
  )

  console.log("OK")
})()
