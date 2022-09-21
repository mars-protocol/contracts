#!/bin/sh

echo what is your name? Note: this is written as userX

read $name 

osmosisd tx sign \
    unsignedTx.json \
    --multisig= #insert multisig address here \
    --from=$name \
    --output-document=`$name`sig.json \
    --chain-id=osmos-test-4