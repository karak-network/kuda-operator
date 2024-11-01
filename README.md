# KUDA Operator Setup Guide

This guide will help you set up and run the KUDA as an Operator.

## System Requirements (Recommended)

- **Memory**: 4 GB

- **Storage**: 30 GB
- **vCPU**: 2
- **OS**: Ubuntu 18.04 and above

## Network Configuration

### Inbound

- **Port**: 8080 (to expose the operator)
- **Public IP/URL**: Used in the `HOST` variable of the environment

### Outbound

- Aggregator server URL
- RPC server URLs (Celestia, Network URLs)

## Karak CLI Installation

Run the following command to install the Karak CLI:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/karak-network/karak-rs/releases/download/karak-cli-v0.2.3/karak-cli-installer.sh | sh
```

## KUDA Operator Installation

Run the following command to install the KUDA operator binary:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/karak-network/kuda-operator/releases/download/v0.2.1/kuda-operator-installer.sh | sh
```

The script will place the binary in the `$HOME/.karak/bin` directory and add this directory to your `$PATH` variable.

## (Optional) Create a local keystore

If you want to use a local keystore, you can create one using the following command:

```bash
karak keypair generate -s local -c secp256k1
```

## Registering the Operator with KUDA

Run the following command to register the operator:


```bash
kuda-operator register \
    --kms local \
    --operator-keystore-path <OPERATOR_KEYSTORE_PATH> \
    --kuda-contract-address <KUDA_CONTRACT_ADDRESS> \
    --core-contract-address <CORE_CONTRACT_ADDRESS> \
    --kuda-rpc_url <KUDA_RPC_URL>
```

- Using AWS KMS:

```bash
kuda-operator register \
    --kms aws \
    --aws-region <AWS_REGION> \
    --aws-access-key-id <AWS_ACCESS_KEY_ID> \
    --aws-secret-access-key <AWS_SECRET_ACCESS_KEY> \
    --aws-operator-key-id <AWS_OPERATOR_KEY_ID> \
    --kuda-contract-address <KUDA_CONTRACT_ADDRESS> \
    --core-contract-address <CORE_CONTRACT_ADDRESS> \
    --kuda-rpc_url <KUDA_RPC_URL>
```

Alternatively, you can put those arguments in an `.env` file or directly export to your environment and run:

```bash
kuda-operator register
```

## Create vault(s)

Run the following command to create a vault:

- Using local keystore:

```bash
karak operator create-vault \
    --assets <ASSETS> \
    --core-address <CORE_ADDRESS> \
    --secp256k1-keystore-path <KEYSTORE_PATH> \
    --rpc-url <RPC_URL>
```

where `<ASSETS>` is a comma-separated list of asset addresses.

For Sepolia, you can use these addresses:

1. Core contract address: `0xb3E2dA61df98E44457190383e1FF13e1ea13280b`

2. Allow listed assets:
    - `0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238`
    - `0x8c843B3A8e9A99680b7611612998799966141841`
    - `0xac8910BEf6c73d30B79e7045ea4fB43fF94833eE`
    - `0xf0091d2b18BabAE32A1B24944f653e69Ac99b7d2`

## (Optional) Deposit to vault

Run the following command to deposit to a vault:

```bash
karak operator deposit-to-vault \
    --vault-address <VAULT_ADDRESS> \
    --amount <AMOUNT> \
    --secp256k1-keystore-path <KEYSTORE_PATH>
    --rpc-url <RPC_URL>
```

where `<VAULT_ADDRESS>` is one of the vault addresses created in the previous step.

Note that you'll need to own at least `AMOUNT` of the asset to deposit.
You can get some of the USDC asset (`0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238`) from [here](https://faucet.circle.com/).
For the other assets, you can mint them yourself.

<!-- TODO: Add mint command -->

## Stake the vault to KUDA

### Request update stake

First, we request an update stake by running:

```bash
karak operator request-stake-update \
    --vault-address <VAULT_ADDRESS> \
    --dss-address <KUDA_ADDRESS> \
    --stake-update-type stake \
    --core-address <CORE_ADDRESS> \
    --secp256k1-keystore-path <KEYSTORE_PATH> \
    --rpc-url <RPC_URL>
```

This command will return a nonce and a start timestamp in the output.

### Finalize stake update

Then, we finalize the stake update by running:

```bash
karak operator finalize-stake-update \
    --vault-address <VAULT_ADDRESS> \
    --dss-address <KUDA_ADDRESS> \
    --stake-update-type stake \
    --nonce <NONCE> \
    --start-timestamp <START_TIMESTAMP> \
    --core-address <CORE_ADDRESS> \
    --secp256k1-keystore-path <KEYSTORE_PATH> \
    --rpc-url <RPC_URL>
```

where

- `<VAULT_ADDRESS>` is one of the vault addresses created earlier
- `<NONCE>` is the nonce returned from the previous command.
- `<START_TIMESTAMP>` is the start timestamp returned from the previous command.

For Sepolia, you can use these addresses:

- `KUDA_ADDRESS`: `0x0e64c3c675dae7537A9fC1E925E2a87e164f7f53`
- `CORE_ADDRESS`: `0xb3E2dA61df98E44457190383e1FF13e1ea13280b`

Note: You can also use AWS KMS instead of a local keystore. Run

```bash
karak operator --help
```

to see all the available options.

## Deployment

Fill out the `compose.yml` or an `.env` file with the following environment variables:

```yaml
AGGREGATOR_URL: <URL of Aggregator server>
KUDA_RPC_URL: <URL of RPC (Sepolia or Mainnet)>
CELESTIA_RPC_URL: <RPC URL of Celestia>
CELESTIA_AUTH_TOKEN: <TOKEN from Celestia>
KMS: <'aws' for AWS KMS or 'keystore' for local keystore>
AWS_REGION: <AWS region, e.g., 'ap-south-1'>
AWS_ACCESS_KEY_ID: <AWS Access Key>
AWS_SECRET_ACCESS_KEY: <AWS Secret Key>
AWS_OPERATOR_KEY_ID: <AWS KMS Key ID for operator>
AWS_EIP4844_KEY_ID: <AWS KMS Key ID for EIP4844>
OPERATOR_KEYSTORE_PATH: <Path to keystore if using 'keystore'>
OPERATOR_KEYSTORE_PASSWORD: <Keystore password>
OPERATOR_PRIVATE_KEY: <Raw private key if using local>
EIP4844_KEYSTORE_PATH: <Path to EIP4844 keystore if using 'keystore'>
EIP4844_KEYSTORE_PASSWORD: <EIP4844 keystore password>
EIP4844_PRIVATE_KEY: <Raw private key if using local>
EIP4844_TO_ADDRESS: <ERC20 address>
EIP4844_RPC_URL: <RPC URL of Network (Sepolia or Mainnet)>
EIP4844_BEACON_URL: <RPC URL of Network (Sepolia or Mainnet)>
RUST_LOG: "info" (Other log levels: error, debug, warn, trace)
```

The aggregator URL for Sepolia is `http://35.154.70.183:8081/`.

### Run the Docker Container

```bash
docker compose up --detach
```

### Shut Down the Docker Container

```bash
docker compose down
```

## Running the Binary

Alternatively, you can run the KUDA operator binary directly:

1. Fill out the environment variables in an `.env` file.
2. Run

```bash
source .env
```

3. Run the binary:

```bash
kuda-operator run
```

---

That's it! You're all set to run the KUDA operator. If you encounter any issues, please refer to the documentation or raise an issue on our GitHub repository.
