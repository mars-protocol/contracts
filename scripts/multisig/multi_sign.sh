#!/bin/sh

echo who signed the multisig? Note: This is written as userX

read name1 

read name2 

read name3 

echo $name1, $name2, $name3

echo name this transaction

read txname

osmosisd tx multisign \
    unsignedTx.json \
    multi \
    `$name1`sig.json `$name2`sig.json `$name3`sig.json \
    --output-document=`$txname`signedTx.json \
    --chain-id=osmo-test-4

