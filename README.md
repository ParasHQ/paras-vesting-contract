PARAS Vesting Contract
==============

## Building this contract
```bash
yarn build
```

## Using this contract

### Quickest deploy
```bash
yarn dev
```

## Testing
To test run:
```bash
yarn test
```

# Contract functions

## View methods

### Get Recipient

```
recipient()
```

### Get Amount

```
amount()
```

### Get Amount claimed

```
amount_claimed()
```

### Get cliff (start + cliff) in nano seconds

```
cliff()
```

### Get start (in nano seconds)

```
start()
```

### Get duration (in nano seconds)

```
duration()
```

### Get revocable

```
revocable()
```

###  Get releasable_amount (amount releasable to recipient at current blockchain timestamp)
```
releasable_amount()
```

### Get amount_vested (total amount vested at current blockchain timestamp)

```
calculate_amount_vested()
```

## Call methods

### New 
```
near call --networkId testnet --accountId alice.testnet contract_id new '{"owner":"alice.testnet.testnet","recipient":"bob.testnet","token":"ft.paras.testnet","amount":"500000000000000000000000000000","start":1622505600000000000,"duration":63072000000000000,"cliff":15552000000000000,"revocable":false}'
```

NOTE: after calling new(), do ft_transfer of PARAS to vesting_contract\
NOTE: the recipient must register on PARAS FT contract to obtain tokens\

### Claim vested

```
claim_vested()
```

### Revoke - Owner Only
```
revoke({"recipient":"alice.testnet"})
```
