# Swap arbitrage
## Contract
#### mainnet
|title|content|
|--|--|
|code id|2439|
|swap-arbitrage|terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj|
|vaults|terra1tgsex5ncsutdpl202mhgmldwtcp2w8nyg6a5gq|

#### testnet
|title|content|
|--|--|
|code id|35831|
|swap-arbitrage|terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a|
|vaults|terra1lx3r6tyjtc6naqhrwxguyfju42z77ujx4jcnfa|

## Command
### instantiate
```
{
    "commission_rate": "0.1",
    "deposit_asset_info": {
        "native_token": {
            "denom": "uusd"
        }
    },
    "asset_info": {
        "token": {
            "contract_addr": "terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl"
        }
    },
    "symbol": "SWAP",
    "token_code_id": 148,
    "router_addr": "terra1c58wrdkyc0ynvvxcv834kz65nfsxmw2w0pwusq",
    "market_money_addr": "terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal"
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