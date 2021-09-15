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
near call dev-1631684538328-15645042144806 --accountId dev-1631684538328-15645042144806 new '{"owner":"dev-1631684538328-15645042144806", "recipient":"rng.testnet","token":"dev-1631277489384-75412609538902","amount":"1250000000000000000000000","start":"1629055854000000000", "duration":"31556952000000000", "cliff_duration":"0", "revocable":false}'
```

NOTE: after calling new(), do ft_transfer of PARAS to vesting_contract\
NOTE: the recipient must register on PARAS FT contract to obtain tokens\

### Claim vested

```
claim_vested()
```

### Revoke - Owner Only (revocable == true)
```
revoke({"recipient":"alice.testnet"})
```
