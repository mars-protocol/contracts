# Neutron Multisig Overview

The multisig on Neutron is set to have 5 multisig holders with a threshold of 3, meaning that 3 signatures are needed for any transaction to pass.

## Set up Neutrond

Neutrond is the daemon for the neutron blockchain. To install, follow [this documentation](https://docs.neutron.org/neutron/build-and-run/neutron-build).

## Set up individual account as a multisig signer on your local network

1. Create the account - to use a consistent naming, we will use [name]\_ntrn e.g. dane_ntrn (similarly on other chains e.g. dane_osmo, dane_mars). It is up to the signer if they wish to use a Ledger or other hardware wallet - or not.

The benefit is that you will be more secure of a signer
The downsides are that:
a. Some Ledgers are not able to sign large messages such as contract uploads
b. If you are travelling a lot it's best to leave your hardware wallet at home in a secure place, and so if this is the case it might actually be more secure to have a hot wallet as hardware wallets are easily recognisable in airport security etc.

```bash
neutrond keys add [name]
```

2. Note down the mnemonic - it is important that you are able to recover this account as a multisig signer.

3. Send a small amount of funds to the address to register it. In testnet you can do this by visiting the facuet [here](https://t.me/+SyhWrlnwfCw2NGM6)

## Set up the multisig on your local network

_Steps 2-4 must be completed by ALL multisig holders to properly set up their local keyring in their machine._

1. Generate the public keys of each of the 5 multisig holder's wallets. In order to generate a public key, the wallet must be active and have made at least one transaction on the specified network to return a public key.

To do a send transaction of 1 NTRN to anoter account you can use the command:

```bash
neutrond tx bank send [name]_ntrn [to_address] 1000000untrn --node=[rpc node] --chain-id=[chain id]
```

Note for testnet node you can use https://testnet-neutron-rpc.marsprotocol.io:443 and chain-id pion-1

Query the public key:

```bash
neutrond query account [address] --node=[node_URL]
```

2. Add each public key to the keys list in your local network.

   ```bash
   neutrond keys add [name] --pubkey=[pubkey]
   ```

   Note: The pubkey must be entered with the same syntax as shown in Step 1.

3. Generate the multisig.

   ```bash
   neutrond keys add neutron_multisig \
     --multisig=[name1],[name2],[name3],[name4],[name5] \
     --multisig-threshold=3
   ```

4. Assert that it was completed correctly.

   ```bash
   neutrond keys show neutron_multisig
   ```

5. Update the config with the new mutlisig address in `red-bank/scripts/deploy/neutron/config`, which will set the owner and admin of the smart contracts to the multisig upon deployment.

## Signing a TX with the multisig - Testnet Migrate Msg Example

**Every multisig holder is responsible for verifying the contract's newly uploaded code for every migrate msg.**

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond

## Signing a TX with the multisig - Testnet Execute Msg Example

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond

## Examples of Execute Args

Refer to the osmosis readme, examples are the same but replacing osmosisd with neutrond
