// Taken from mnemonics.json in LocalOsmosis repo
// All test users have uion & uosmo balances

export interface walletDataType {
  address: string;
  name: string;
  mnemonic: string;
  pubkey: { '@type': string; key: string };
}

const walletData: walletDataType[] = [
  {
    name: 'validator',
    address: 'osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'AkNVLtIlk2c3zweQXS6jyVshzVhAy0M59crUeksc2pak' },
    mnemonic:
      'satisfy adjust timber high purchase tuition stool faith fine install that you unaware feed domain license impose boss human eager hat rent enjoy dawn',
  },
  {
    name: 'test1',
    address: 'osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'AuwYyCUBxQiBGSUWebU46c+OrlApVsyGLHd4qhSDZeiG' },
    mnemonic:
      'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius',
  },
  {
    name: 'test2',
    address: 'osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A2G5GnZLlHyxQJUI6LW2ww1lnFEBy+3CCl8LsK2OY6Tj' },
    mnemonic:
      'quality vacuum heart guard buzz spike sight swarm shove special gym robust assume sudden deposit grid alcohol choice devote leader tilt noodle tide penalty',
  },
  {
    name: 'test3',
    address: 'osmo1qwexv7c6sm95lwhzn9027vyu2ccneaqad4w8ka',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'ApNMBAr8lFRS6DaOKXgGXFcrpf78KHyqPvRCLZrM0Zzg' },
    mnemonic:
      'symbol force gallery make bulk round subway violin worry mixture penalty kingdom boring survey tool fringe patrol sausage hard admit remember broken alien absorb',
  },
  {
    name: 'test4',
    address: 'osmo14hcxlnwlqtq75ttaxf674vk6mafspg8xwgnn53',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A0RRfnW/yIHOgFjGpknpT/j19OP3YMsXj6OhuCCHfyu6' },
    mnemonic:
      'bounce success option birth apple portion aunt rural episode solution hockey pencil lend session cause hedgehog slender journey system canvas decorate razor catch empty',
  },
  {
    name: 'test5',
    address: 'osmo12rr534cer5c0vj53eq4y32lcwguyy7nndt0u2t',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A5sEEVq3yKGF/pDihGjtSe3SElOd05zXzMxCBPMAhspC' },
    mnemonic:
      'second render cat sing soup reward cluster island bench diet lumber grocery repeat balcony perfect diesel stumble piano distance caught occur example ozone loyal',
  },
  {
    name: 'test6',
    address: 'osmo1nt33cjd5auzh36syym6azgc8tve0jlvklnq7jq',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'AweL0IVkZAHjmdSPJucxcln3AcPuMHD4EcnjKBFLZkcA' },
    mnemonic:
      'spatial forest elevator battle also spoon fun skirt flight initial nasty transfer glory palm drama gossip remove fan joke shove label dune debate quick',
  },
  {
    name: 'test7',
    address: 'osmo10qfrpash5g2vk3hppvu45x0g860czur8ff5yx0',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A5aDi6tH57PDZossksf820HI+kVGO5etFqjGbFw/tACu' },
    mnemonic:
      'noble width taxi input there patrol clown public spell aunt wish punch moment will misery eight excess arena pen turtle minimum grain vague inmate',
  },
  {
    name: 'test8',
    address: 'osmo1f4tvsdukfwh6s9swrc24gkuz23tp8pd3e9r5fa',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'AgZffLI+SEDH5qCrGoG4HjPy8AIDVmjGZJzy7L3YNkb9' },
    mnemonic:
      'cream sport mango believe inhale text fish rely elegant below earth april wall rug ritual blossom cherry detail length blind digital proof identify ride',
  },
  {
    name: 'test9',
    address: 'osmo1myv43sqgnj5sm4zl98ftl45af9cfzk7nhjxjqh',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A65FjujcdnaFQutpnfkj82QSKtYOMBJPaW4pfTiERwMu' },
    mnemonic:
      'index light average senior silent limit usual local involve delay update rack cause inmate wall render magnet common feature laundry exact casual resource hundred',
  },
  {
    name: 'test10',
    address: 'osmo14gs9zqh8m49yy9kscjqu9h72exyf295afg6kgk',
    pubkey: { '@type': '/cosmos.crypto.secp256k1.PubKey', key: 'A2Kc6ERRH6B4REjY6ryTO+ZdNbxuJATDVKXA89irZpKO' },
    mnemonic:
      'prefer forget visit mistake mixture feel eyebrow autumn shop pair address airport diesel street pass vague innocent poem method awful require hurry unhappy shoulder',
  },
];

export const testWallet1 = walletData[1];
export const testWallet2 = walletData[2];
