#!/bin/sh

#step 3: create multisignature 
echo what is the multisig address? 

read multisig

echo enter signer 1 name 

read name1

echo enter signer 2 name 

read name2

echo enter signer 3 name 

read name3

echo enter signedTx file name 

read signedTx

osmosisd tx multisign \
    unsignedTx.json \
    $multisig \
    `$name1`sig.json `$name2`sig.json `$name3`sig.json \
    --output-document=`$signedTx`.json \
    --chain-id=osmo-test-4