#!/bin/sh

# to generate a public key, each person needs to run 'osmosisd add key [name]'

echo what is user1 PubKey?
read User1Key

echo what is user2 PubKey?
read User2Key

echo what is user3 PubKey?
read User3Key

echo what is user4 PubKey?
read User4Key

echo what is user5 PubKey?
read User5Key

osmosisd keys add user1 --pubkey=$User1Key
osmosisd keys add user2 --pubkey=$User2Key
osmosisd keys add user3 --pubkey=$User3Key
osmosisd keys add user4 --pubkey=$User4Key
osmosisd keys add user5 --pubkey=$User5Key

osmosisd keys add mars_testnet_multisig --multisig=user1,user2,user3,user4,user5 --multisig-threshold=3

osmosisd keys show mars_testnet_multisig