import {
  LCDClient,
  LegacyAminoMultisigPublicKey,
  MsgExecuteContract,
  SimplePublicKey
} from "@terra-money/terra.js"
import { writeFileSync } from "fs"
import 'dotenv/config.js'

// CONSTS

if (!(process.env.CHAIN_ID
  && process.env.LCD_CLIENT_URL
  && process.env.MULTISIG_PUBLIC_KEYS
  && process.env.MULTISIG_THRESHOLD
  && process.env.CONTRACT_ADDRESS
  && process.env.TRANSACTION_DESCRIPTION
  && process.env.EXECUTE_MSG
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
// Transaction details:
// The address that the tx will be sent to
const CONTRACT_ADDRESS = process.env.CONTRACT_ADDRESS
// A description of the transaction
const TRANSACTION_DESCRIPTION = process.env.TRANSACTION_DESCRIPTION
// A JSON object of the operation to be executed on the contract
const EXECUTE_MSG = JSON.parse(process.env.EXECUTE_MSG);

// MAIN

(async () => {
  const terra = new LCDClient({
    URL: LCD_CLIENT_URL,
    chainID: CHAIN_ID
  })

  // Create an unsigned tx
  const multisigPubKey = new LegacyAminoMultisigPublicKey(MULTISIG_THRESHOLD, MULTISIG_PUBLIC_KEYS)

  const multisigAddress = multisigPubKey.address()
  console.log("multisig:", multisigAddress)

  const accInfo = await terra.auth.accountInfo(multisigAddress)

  const tx = await terra.tx.create(
    [
      {
        address: multisigAddress,
        sequenceNumber: accInfo.getSequenceNumber(),
        publicKey: accInfo.getPublicKey(),
      },
    ],
    {
      msgs: [
        new MsgExecuteContract(
          multisigAddress,
          CONTRACT_ADDRESS,
          EXECUTE_MSG
        )
      ]
    }
  )

  // The unsigned tx file should be distributed to the multisig key holders
  const unsignedTxFilename = `${TRANSACTION_DESCRIPTION}_unsigned.json`

  writeFileSync(unsignedTxFilename, JSON.stringify(tx.toData()))

  // Prints a command that should be run by the multisig key holders to generate signatures
  console.log(`
Multisig transaction signing instructions

Multisig: ${multisigAddress}
Transaction: ${TRANSACTION_DESCRIPTION}

- Set \`signing_address\` to your address that you are signing the transaction with
- Set \`first_name\` to your first name
- If your address is on a Ledger hardware device, make sure the Terra app is open
- Run the signing command
- Return the signed json file

\`\`\`
signing_address=terra1...

first_name=...

terrad tx sign ${unsignedTxFilename} \\
  --multisig ${multisigAddress} \\
  --from \$signing_address \\
  --chain-id ${terra.config.chainID} \\
  --offline \\
  --account-number ${accInfo.getAccountNumber()} \\
  --sequence ${accInfo.getSequenceNumber()} \\
  --output-document ${TRANSACTION_DESCRIPTION}_signed_\${first_name}.json
\`\`\`
`)

  // Run `broadcast_tx.ts` to aggregate at least K of N signatures and broadcast the signed tx to the network
})()
