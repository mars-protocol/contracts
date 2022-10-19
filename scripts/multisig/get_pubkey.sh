#!/bin/sh

echo Has your wallet been used on testnet or mainnet? 

read network

if (($network == testnet));
then
    echo what is your wallet address? 
    read testnet
    echo Go to site: https://lcd-test.osmosis.zone/cosmos/auth/v1beta1/accounts/$testnet
elif (($network == mainnet));
then
    echo what is your wallet address? 
    read $mainnet 
    echo Go to site: https://osmosis-api.polkachu.com/cosmos/auth/v1beta1/accounts/$mainnet
else 
    echo wrong network specified
fi
