# Multisig overview: 
The following commands will set up a multisig account on osmosis using osmosisd. 
This multisig is set to have 5 multisig holders and needs 3 signatures for a transaction to go through. 

## Set up osmosisd 
1. Run the following command and follow the onscreen instructions: 
```
curl -sL https://get.osmosis.zone/install > i.py && python3 i.py
``` 

If you are running on an Apple M1 chip, run: 
```
git clone https://github.com/osmosis-labs/osmosis.git
make build
sudo cp build/osmosisd /usr/local/bin
```

2. Update the system (If on Linux): 
```
sudo apt update
sudo apt upgrade --yes
```

3. Install Build Requirements 
```
sudo apt install git build-essential ufw curl jq snapd --yes
wget -q -O - https://git.io/vQhTU | bash -s -- --version 1.17.2
```

4. Install osmosis binary: 
```
cd $HOME
git clone https://github.com/osmosis-labs/osmosis
cd osmosis

git checkout v11.0.1

make install
```

## Set up multisig: 

1. Generate indivdual public keys. Each multisig holder needs to run: 
```
yarn get-pubkey
```
Note: Your wallet must be active and have made atleast one transaction to return a public key. 

This will return a JSON that had your pubkey. Copy your pubkey in the following format: 
```JSON
'{
    "@type": "/cosmos.crypto.secp256k1.PubKey", 
    "key": "alkfjadfyeohiskvbskjas,jdla"
}'
```

2. To create the multisig, each multisig owner's pubkey needs to be gathered. Then run the following:
```
yarn create-multisig 
```

3. Update the config with the new multisig address in scripts/deploy/osmosis/config.