import {
  isTxError,
  LCDClient,
  LegacyAminoMultisigPublicKey,
  SimplePublicKey,
  Tx
} from "@terra-money/terra.js"
import { SignatureV2 } from "@terra-money/terra.js/dist/core/SignatureV2.js"
import { MultiSignature } from "@terra-money/terra.js/dist/core/MultiSignature.js"
import { readFileSync } from "fs"
import * as path from "path"
import * as glob from "glob"
import 'dotenv/config.js'

// CONSTS

if (!(process.env.CHAIN_ID
  && process.env.LCD_CLIENT_URL
  && process.env.MULTISIG_PUBLIC_KEYS
  && process.env.MULTISIG_THRESHOLD
  && process.env.TRANSACTION_DESCRIPTION
)) {
  throw new Error("One or more required environment variables are missing")
}

// Terra network details:
const CHAIN_ID = process.env.CHAIN_ID
const LCD_CLIENT_URL = process.env.LCD_CLIENT_URL
// Multisig details:
const MULTISIG_PUBLIC_KEYS = process.env.MULTISIG_PUBLIC_KEYS
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
const MULTISIG_THRESHOLD = parseInt(process.env.MULTISIG_THRESHOLD)

// A description of the transaction
const TRANSACTION_DESCRIPTION = process.env.TRANSACTION_DESCRIPTION;

// MAIN

(async () => {
  const terra = new LCDClient({
    URL: LCD_CLIENT_URL,
    chainID: CHAIN_ID
  })

  const multisigPubKey = new LegacyAminoMultisigPublicKey(MULTISIG_THRESHOLD, MULTISIG_PUBLIC_KEYS)
  const multisigAddress = multisigPubKey.address()
  console.log("multisig:", multisigAddress)
  const multisig = new MultiSignature(multisigPubKey)

  const tx = Tx.fromData(JSON.parse(readFileSync(`${TRANSACTION_DESCRIPTION}_unsigned.json`).toString()))

  // Sign the tx using the signatures from the multisig key holders
  const signatureFiles = glob.sync(path.join(__dirname, `${TRANSACTION_DESCRIPTION}_signed_*.json`))
  console.log(signatureFiles)

  const signatures = signatureFiles.map(
    file => SignatureV2.fromData(
      JSON.parse(
        readFileSync(file).toString()
      ).signatures[0]
    )
  )

  multisig.appendSignatureV2s(signatures)

  const accInfo = await terra.auth.accountInfo(multisigAddress)

  tx.appendSignatures([
    new SignatureV2(
      multisigPubKey,
      multisig.toSignatureDescriptor(),
      accInfo.getSequenceNumber()
    )
  ])

  // Broadcast the tx
  const result = await terra.tx.broadcast(tx)
  if (isTxError(result)) {
    throw new Error(result.raw_log)
  }
  console.log(`https://finder.terra.money/${CHAIN_ID}/tx/${result.txhash}`)
})()
