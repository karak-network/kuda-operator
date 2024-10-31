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

## Prerequisites
- Docker
- Geth (to create keystore wallet)

## Installation

Run the following command to download the binaries:

```bash
 curl --proto '=https' --tlsv1.2 -LsSf https://github.com/karak-network/kuda-operator/releases/download/v0.1.0/kuda-operator-installer.sh | sh
````

The script will place these binaries in the `$HOME/.karak/bin` directory and add this directory to your `$PATH` variable.

## Registering the Operator with KUDA

Run the following command to register the operator:

- Using local keystore:

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

## Registering the Operator with Core

Follow the steps [here](https://docs.karak.network/operators/registration) for Karak Operator registration.

## Deployment

Fill out the `compose.yml` with the following environment variables:

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

### Run the Docker Container

    docker compose up --detach

### Shut Down the Docker Container

    docker compose down

## Running the Binary

Alternatively, you can run the KUDA operator binary directly:

1. Fill out the environment variables in an `.env` file.
2. run
```bash
source .env
 ```
3. Run the binary:

```bash
kuda-operator run
```

---

That's it! You're all set to run the KUDA operator. If you encounter any issues, please refer to the documentation or raise an issue on our GitHub repository.
