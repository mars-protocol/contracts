import { getSigningOsmosisClient } from 'osmojs';
import { SigningStargateClient } from '@cosmjs/stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { walletDataType } from './test-wallets';
import { Network, networks } from './config';
import { Slip10RawIndex } from '@cosmjs/crypto';
import { MsgStoreCode } from 'cosmjs-types/cosmwasm/wasm/v1/tx';

const hdPath = [
  Slip10RawIndex.hardened(44),
  Slip10RawIndex.hardened(118),
  Slip10RawIndex.hardened(0),
  Slip10RawIndex.normal(0),
  Slip10RawIndex.normal(0),
];

type ClientGetter = (wallet: walletDataType) => Promise<SigningStargateClient>;

export const getOsmosisClient: ClientGetter = async (wallet) => {
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(wallet.mnemonic, {
    prefix: networks[Network.OSMOSIS].bech32Prefix,
    hdPaths: [hdPath],
  });

  const client = await getSigningOsmosisClient({
    rpcEndpoint: networks[Network.OSMOSIS].localRpcEndpoint,
    signer,
  });

  client.registry.register('/cosmwasm.wasm.v1.MsgStoreCode', MsgStoreCode);
  return client;
};
