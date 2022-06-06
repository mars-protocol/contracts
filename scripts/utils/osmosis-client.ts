import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Slip10RawIndex } from '@cosmjs/crypto';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { SigningStargateClient } from '@cosmjs/stargate';
import { MsgInstantiateContract, MsgStoreCode } from 'cosmjs-types/cosmwasm/wasm/v1/tx';
import { getSigningOsmosisClient } from 'osmojs';
import { Network, networks } from './config';
import { walletDataType } from './test-wallets';

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
  client.registry.register('/cosmwasm.wasm.v1.MsgInstantiateContract', MsgInstantiateContract);

  return client;
};

/* Separate client needed as querying not available in signed Osmosis client at the moment */
export const getQueryClient: () => Promise<CosmWasmClient> = async () => {
  return await CosmWasmClient.connect(networks[Network.OSMOSIS].localRpcEndpoint);
};
