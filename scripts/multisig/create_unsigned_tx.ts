import {
  LCDClient,
  LegacyAminoMultisigPublicKey,
  MsgExecuteContract,
  SimplePublicKey
} from "@terra-money/terra.js"
import { writeFileSync } from "fs"
import 'dotenv/config.js'

// CONSTS

// Required environment variables:
// Terra network details:
const CHAIN_ID = process.env.CHAIN_ID!
const LCD_CLIENT_URL = process.env.LCD_CLIENT_URL!
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
// Transaction details:
// The address that the tx will be sent to
const CONTRACT_ADDRESS = process.env.CONTRACT_ADDRESS!
// A JSON object of the operation to be executed on the contract
const EXECUTE_MSG = JSON.parse(process.env.EXECUTE_MSG!);

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
  const unsignedTx = "unsigned_tx.json"

  writeFileSync(unsignedTx, JSON.stringify(tx.toData()))

  // Prints a command that should be run by the multisig key holders to generate signatures
  // TODO add Ledger support
  console.log(`
# Instructions to sign a tx for multisig ${multisigAddress}:
# - 1. set \`multisig\` to the name of the multisig in terrad (check the name with \`terrad keys list\`):
multisig=...

# - 2. set \`from\` to your address that is a key to the multisig, or its name in terrad:
from=terra1...

# - 3. run the signing command:
terrad tx sign ${unsignedTx} \\
  --multisig \$multisig \\
  --from \$from \\
  --chain-id ${terra.config.chainID} \\
  --offline \\
  --account-number ${accInfo.getAccountNumber()} \\
  --sequence ${accInfo.getSequenceNumber()} \\
  --output-document \${from}_sig.json
`)

  // Run `broadcast_tx.ts` to aggregate at least K of N signatures and broadcast the signed tx to the network
})()
