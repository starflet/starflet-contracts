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
|code id|11079|
|contract|terra1kv3uuf3kddfqqqjhzk7xze7qeu52lep4mcsuc2|

## Command
### InstantiateMsg
```
{}
```

### Execute Create Planet
```
{
    "add_planet": {
        "contract_addr": "terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj",
        "title": "swap arbitrage",
        "description": "Run arbitrage using LUNA<>UST market between native swap and terraswap."
    }
}
```

### Execute Edit Planet
```
{
    "edit_planet": {
        "contract_addr": "terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj",
        "description": "Run arbitrage using LUNA<>UST market between native swap and terraswap"
    }
}
```

### Execute Remove Planet
```
{
    "remove_planet": {
        "contract_addr": "terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj"
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
        "start_after": "terra1ndvfjs47eax9yxkc5tge2awlahswry3tg76zvj",
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