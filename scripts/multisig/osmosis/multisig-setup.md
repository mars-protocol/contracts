# Osmosis Multisig Overview

The multisig on Osmosis is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

The Osmosis multisig being used for this project is `osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n`

## Set up Osmosisd

Osmosisd is the daemon for the osmosis blockchain. To install, follow [this documentation](https://docs.osmosis.zone/osmosis-core/osmosisd/).

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._

1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

   ```bash
   osmosisd query account [address] --node=[node_URL]
   ```

2. Add each public key to the keys list in your local network.

   ```bash
   osmosisd keys add [name] --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.

   ```bash
   osmosisd keys add osmosis_multisig \
     --multisig=[name1],[name2],[name3],[name4],[name5] \
     --multisig-threshold=3
   ```

4. Assert that it was completed correctly.

   ```bash
   osmosisd keys show osmosis_multisig
   ```

5. Update the config with the new mutlisig address in `rover/scripts/deploy/osmosis/config`, which will set the owner and admin of the smart contracts to the multisig upon deployment.
