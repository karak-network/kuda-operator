# KUDA operator

## Download the installer

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/karak-network/kuda-operator/releases/download/v0.1.0/kuda-operator-installer.sh | sh
```

## Register your operator (Sepolia)

```shell
kuda-operator-register --signer-type local --keystore <KEYSTORE_PATH> --rpc-url <rpc-url> --kuda-address 0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b --core-address 0xb3E2dA61df98E44457190383e1FF13e1ea13280b --token-address 0x8c843B3A8e9A99680b7611612998799966141841 --vault-impl 0x6dAB3085943Adb6C6785f51bab7EDc1f9e9B1077 --amount <amount>
```

## Run the operator

You'll need to provide the following environment variables:

```shell
AGGREGATOR_URL=http://3.110.29.45:8081/
KUDA_CONTRACT_ADDRESS=0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b
CORE_CONTRACT_ADDRESS=0xb3E2dA61df98E44457190383e1FF13e1ea13280b
CELESTIA_RPC_URL=
CELESTIA_AUTH_TOKEN=

SIGNER_TYPE=local/aws

AWS_REGION=
AWS_ACCESS_KEY_ID=
AWS_SECRET_ACCESS_KEY=
AWS_OPERATOR_KEY_ID=
AWS_EIP4844_KEY_ID=

OPERATOR_KEYSTORE_PATH=
OPERATOR_KEYSTORE_PASSWORD=
EIP4844_KEYSTORE_PATH=
EIP4844_KEYSTORE_PASSWORD=

EIP4844_TO_ADDRESS=
EIP4844_RPC_URL=
EIP4844_BEACON_URL=

RUST_LOG= info
```

If you're using the `local` signer, you don't need to provide the AWS environment variables. If you're using the `aws` signer, you don't need to provide the local environment variables.

Then you can run the operator:

```shell
kuda-operator
```

## Docker

You can also run the operator using docker:

```shell
docker pull ghcr.io/karak-network/kuda-operator:latest
```

Fill out the environment variables in the `docker-compose.yml` file and run the operator:

```shell
docker-compose up -d
```
