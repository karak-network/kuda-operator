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
- Rustc, Cargo (latest stable version)
- Geth (to create keystore wallet)

## Installation

Run the following command to download the binaries:

    curl --proto '=https' --tlsv1.2 -LsSf https://github.com/karak-network/kuda-operator/releases/download/v0.1.0/kuda-operator-installer.sh | sh

This installer will install two binaries:
1. `kuda-operator`: The main KUDA operator binary
2. `kuda-operator-register`: CLI used for registration

The script will place these binaries in the `$HOME/.karak` directory and add this directory to your `$PATH` variable.

## Registering the Operator

### With Keystore
```bash
    kuda-operator-register --signer-type keystore --keystore {path-to-keystore} --rpc-url {NETWORK_RPC_URL} \
    --kuda-address 0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b --core-address 0xb3E2dA61df98E44457190383e1FF13e1ea13280b \
    --token-address 0x8c843B3A8e9A99680b7611612998799966141841 --vault-impl 0x6dAB3085943Adb6C6785f51bab7EDc1f9e9B1077 --amount 10000
```
### With AWS KMS

```bash
    kuda-operator-register --signer-type aws --aws-region {AWS_REGION} --aws-access-key-id {AWS_ACCESS_KEY} \
    --aws-secret-access-key {AWS_ACCESS_KEY} --aws-operator-key-id 08251e9b-0159-43f4-af82-721ff458b8dd --rpc-url {NETWORK_RPC_URL} \
    --kuda-address 0x03Fe1aaDfc42DF23947A922aA924caCDfa16832b --core-address 0xb3E2dA61df98E44457190383e1FF13e1ea13280b \
    --token-address 0x8c843B3A8e9A99680b7611612998799966141841 --vault-impl 0x6dAB3085943Adb6C6785f51bab7EDc1f9e9B1077 --amount 10000
```
## Key Management

### Type 1: Keystore
To create a new account, run the following command:

```bash
   geth account new
```

Use the keystore file path in the `compose.yml`. For example, if the path is:

    /User/name/Library/Ethereum/keystore/UTC--2024-10-10T11-33-57.493706000Z--41001411

Your `volumes` configuration would look like this:

    - volumes:
      - /User/name/Library/Ethereum/keystore/UTC--2024-10-10T11-33-57.493706000Z--4100141:/keystore

### Type 2: AWS KMS
Set the following environment variables:
- `AWS_REGION`
- `AWS_ACCESS_KEY_ID`
- `AWS_OPERATOR_KEY_ID`
- `AWS_EIP4844_KEY_ID`

For more information, refer to the [AWS KMS Documentation](https://docs.aws.amazon.com/kms/latest/developerguide/overview.html).

## Docker Setup

Fill out the `compose.yml` with the following environment variables:

    AGGREGATOR_URL: <URL of Aggregator server>
    KUDA_RPC_URL: <URL of RPC (Sepolia or Mainnet)>
    CELESTIA_RPC_URL: <RPC URL of Celestia>
    CELESTIA_AUTH_TOKEN: <TOKEN from Celestia>
    SIGNER_TYPE: <'aws' for AWS KMS or 'keystore' for local keystore>
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

    docker compose up --build --detach

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

       kuda-operator

---

That's it! You're all set to run the KUDA operator. If you encounter any issues, please refer to the documentation or raise an issue on our GitHub repository.
