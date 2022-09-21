## Set up multisig: 

1. Generate indivdual public keys. Each multisig holder needs to run: 
```
osmosisd add key [userX]
```

where X is your assigned number.

2. To create the multisig:  
```
yarn create-multisig 
```

3. Update single_sign.sh & config file in ./deploy/[chainname]/config with the generated multisig address 

## Create transaction with multisig: 

1. Each multisig holder must run: 
```
yarn single-sign 
```
this will generate a [name]sig.json of their signed tx 

2. One holder must gather 3 of the above files and run: 
```
yarn multi- sign 
```