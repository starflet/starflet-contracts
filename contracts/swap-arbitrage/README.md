# Swap arbitrage
## Contract
#### mainnet
|title|content|
|--|--|
|code id|1809|
|swap-arbitrage|terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj|
|vaults|terra1tgsex5ncsutdpl202mhgmldwtcp2w8nyg6a5gq|

#### testnet
|title|content|
|--|--|
|code id|27629|
|swap-arbitrage|terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a|
|vaults|terra1lx3r6tyjtc6naqhrwxguyfju42z77ujx4jcnfa|

## Command
### instantiate
```
{
    "commission_rate": "0.1",
    "asset_info": {
        "native_token": {
            "denom": "uusd"
        }
    },
    "symbol": "SWAP",
    "token_code_id": 148 // cw20
    "terraswap_router": "terra14z80rwpd0alzj4xdtgqdmcqt9wd9xj5ffd60wp" // terraswap router contract
}
```

### Execute bond
```
{
  "bond": {
    "asset": {
      "info": {
        "native_token": {
          "denom": "uusd"
        }
      },
      "amount": "100000000"
    }
  }
}
```

### Execute unbond
```
{
    "send": {
        "amount": "10000000",
        "contract": "terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a", // planet contract address
        "msg": "eyJ1bmJvbmQiOnt9fQ=="
    }
}
```


### Query config
```
{
    "config":{}
}
```

### Query staker_info
```
{
    "staker_info": {
        "staker_addr": "terra1l22flz33tq7fc5g9c6lj7jlxeufcg2q29fpqm8"
    }
}
```