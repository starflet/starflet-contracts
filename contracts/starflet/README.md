# STARFLET


## Contract
#### mainnet
|title|content|
|--|--|
|code id|1463|
|contract|terra1ek9f0zw2xqjn4k70anapdlvmukysvsyyjwteyk|

#### testnet
|title|content|
|--|--|
|code id|33342|
|contract|terra1l90p7fpyqpjp234nas5jzkfnn74f74qyu0gdvk|

## Command
### InstantiateMsg
```
{}
```

### Execute Create Planet
```
{
    "add_planet": {
        "contract_addr": "terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a",
        "title": "swap arbitrage",
        "description": "Run arbitrage using LUNA<>UST market between native swap and terraswap."
    }
}
```

### Execute Edit Planet
```
{
    "edit_planet": {
        "contract_addr": "terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a",
        "description": "Run arbitrage using LUNA<>UST market between native swap and terraswap"
    }
}
```

### Execute Remove Planet
```
{
    "remove_planet": {
        "contract_addr": "terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a"
    }
}
```

### Execute Update Config
```
{
    "update_config": {
        "admin": "terra1xxxx"
    }
}
```

### Query all planets
```
{
    "planets": {
        "start_after": "terra1a3cf7kj0leg9lsk29l2ghh6m6e8n8juy85fp2a",
        "limit": 1
    }
}
```

## Query planet
```
{
    "planet": {
        "planet_contract": "terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj"
    }
}
```