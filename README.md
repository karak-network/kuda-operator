# KUDA operator

## Register your operator (Sepolia)

```shell
cargo run --bin kuda-operator-register -- --private-key <private-key> --rpc-url <rpc-url> --kuda-address 0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b --core-address 0xb3E2dA61df98E44457190383e1FF13e1ea13280b --token-address 0x8c843B3A8e9A99680b7611612998799966141841 --vault-impl 0x6dAB3085943Adb6C6785f51bab7EDc1f9e9B1077 --amount <amount>
```

## Pull the docker image from the registry

```shell
docker pull ghcr.io/karak-network/kuda/operator:main
```

## Run the operator

You'll need to provide the following environment variables:

```yaml
AGGREGATOR_URL: http://3.110.29.45:8081/
CELESTIA_RPC_URL:
CELESTIA_AUTH_TOKEN:
OPERATOR_PRIVATE_KEY:
KUDA_CONTRACT_ADDRESS: 0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b
CORE_CONTRACT_ADDRESS: 0xb3E2dA61df98E44457190383e1FF13e1ea13280b
KUDA_RPC_URL:
EIP4844_PRIVATE_KEY:
EIP4844_TO_ADDRESS:
EIP4844_RPC_URL:
EIP4844_BEACON_URL:

RUST_LOG: info
```
