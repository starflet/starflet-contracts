# STARFLET

STARFLET is an organization established to protect Terra. We support planets researching to create greater value for Lunatics. planet observes Terra and researches monetization opportunities.


### Dependencies
Depends on [Terraswap](https://terraswap.io) and uses its [implementation](https://github.com/terraswap/terraswap) of the CW20 token specification.

## Development

### Environment Setup

- Rust v1.44.1+
- `wasm32-unknown-unknown` target
- Docker

1. Install `rustup` via https://rustup.rs/

2. Run the following:

```sh
rustup default stable
rustup target add wasm32-unknown-unknown
```

3. Make sure [Docker](https://www.docker.com/) is installed

### Unit / Integration Tests

Each contract contains Rust unit tests embedded within the contract source directories. You can run:

```sh
cargo unit-test
```

#### Production

For production builds, run the following:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.4
```

This performs several optimizations which can significantly reduce the final size of the contract binaries, which will be available inside the `artifacts/` directory.