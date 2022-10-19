#!/bin/sh

#step 2: each holder must sign the tx individually
echo what is the unsigned JSON file name on your computer (excluding the '.json')? 

read unsignedTx

echo what is the multisig address?

read multisig

echo what is your wallet address?

read wallet 

echo what is your name?

read name

osmosisd tx sign \
    `$unsignedTx`.json \
    --multisig=$multisig \
    --from=$wallet \
    --output-document=`$name`sig.json \
    --chain-id=osmo-test-4