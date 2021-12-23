# Planet
It is a schema for vaults management, and execute msg must be specified when distributing the actual contract.

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