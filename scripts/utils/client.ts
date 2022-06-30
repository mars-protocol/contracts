import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Network, networks } from './config';
import { walletDataType } from './test-wallets';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

type ClientGetter = (wallet: walletDataType) => Promise<SigningCosmWasmClient>;

export const getCosmWasmClient: ClientGetter = async (wallet) => {
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(wallet.mnemonic, {
    prefix: networks[Network.OSMOSIS].bech32Prefix,
  });
  return await SigningCosmWasmClient.connectWithSigner(networks[Network.OSMOSIS].localRpcEndpoint, signer);
};
