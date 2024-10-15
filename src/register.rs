use std::{io::Write, path::PathBuf};

use alloy::{
    network::{EthereumWallet, TxSigner},
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder},
    signers::{aws::AwsSigner, local::PrivateKeySigner},
};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_kms::config::{Credentials, SharedCredentialsProvider};
use clap::{
    builder::{styling::AnsiColor, Styles},
    Parser,
};
use eyre::Result;
use kuda_operator::{
    contracts::{
        core::{Core, Operator, VaultLib},
        erc20_mintable::ERC20Mintable,
        kuda::Kuda,
        vault::Vault,
    },
    SignerType,
};
use url::Url;

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Green.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Debug, Parser)]
#[command(version, about, styles = styles())]
struct Cli {
    #[arg(short, long)]
    signer_type: SignerType,

    #[arg(long, required_if_eq("signer_type", "aws"), env("AWS_REGION"))]
    aws_region: Option<String>,

    #[arg(long, required_if_eq("signer_type", "aws"), env("AWS_ACCESS_KEY_ID"))]
    aws_access_key_id: Option<String>,

    #[arg(
        long,
        required_if_eq("signer_type", "aws"),
        env("AWS_SECRET_ACCESS_KEY")
    )]
    aws_secret_access_key: Option<String>,

    #[arg(long, required_if_eq("signer_type", "aws"), env("AWS_OPERATOR_KEY_ID"))]
    aws_operator_key_id: Option<String>,

    #[arg(
        short,
        long,
        required_if_eq("signer_type", "local"),
        env("OPERATOR_KEYSTORE_PATH")
    )]
    keystore: Option<PathBuf>,

    #[arg(short, long)]
    rpc_url: Url,

    #[arg(long, env("KUDA_CONTRACT_ADDRESS"))]
    kuda_address: Address,

    #[arg(long, env("CORE_CONTRACT_ADDRESS"))]
    core_address: Address,

    #[arg(short, long, env("TOKEN_ADDRESS"))]
    token_address: Address,

    #[arg(short, long, env("VAULT_IMPL"))]
    vault_impl: Address,

    #[arg(short, long)]
    amount: U256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let keystore = cli.keystore;

    let (operator_wallet, operator_address) = match cli.signer_type {
        SignerType::Local => {
            let keystore = keystore.expect("Keystore must be set when using local signer");
            let signer = local_signer(&keystore)?;
            let address = signer.address();
            (EthereumWallet::from(signer), address)
        }
        SignerType::Aws => {
            let region = cli
                .aws_region
                .expect("AWS region must be set when using AWS signer");
            let access_key_id = cli
                .aws_access_key_id
                .expect("AWS access key ID must be set when using AWS signer");
            let secret_access_key = cli
                .aws_secret_access_key
                .expect("AWS secret access key must be set when using AWS signer");
            let operator_key_id = cli
                .aws_operator_key_id
                .expect("AWS operator key ID must be set when using AWS signer");

            let signer =
                aws_signer(region, &access_key_id, &secret_access_key, operator_key_id).await?;
            let address = signer.address();
            (EthereumWallet::from(signer), address)
        }
    };

    let rpc_url = cli.rpc_url;
    let kuda_address = cli.kuda_address;
    let core_address = cli.core_address;
    let token_address = cli.token_address;
    let vault_impl = cli.vault_impl;
    let amount = cli.amount;

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(operator_wallet)
        .on_http(rpc_url);

    let core_instance = Core::new(core_address, provider.clone());
    let erc20_instance = ERC20Mintable::new(token_address, provider.clone());
    let decimals = erc20_instance.decimals().call().await?._0;
    let name = erc20_instance.name().call().await?._0;
    let symbol = erc20_instance.symbol().call().await?._0;

    let vault_configs = vec![VaultLib::Config {
        asset: token_address,
        decimals,
        operator: operator_address,
        name,
        symbol,
        extraData: Bytes::new(),
    }];

    println!("Deploying vault");
    let vault_address = core_instance
        .deployVaults(vault_configs, vault_impl)
        .send()
        .await?
        .get_receipt()
        .await?
        .inner
        .logs()[3]
        .log_decode::<Core::DeployedVault>()?
        .inner
        .data
        .vault;

    println!("Vault deployed at address: {vault_address}");

    let vault_instance = Vault::new(vault_address, provider.clone());
    println!("Minting tokens");
    erc20_instance
        .mint(operator_address, amount)
        .send()
        .await?
        .get_receipt()
        .await?;

    println!("Approving vault to spend tokens");
    erc20_instance
        .approve(vault_address, amount)
        .send()
        .await?
        .get_receipt()
        .await?;
    println!("Depositing tokens into vault");
    vault_instance
        .deposit_0(amount, operator_address)
        .send()
        .await?
        .get_receipt()
        .await?;

    println!("Submitting operator bond");
    let kuda_instance = Kuda::new(kuda_address, provider.clone());
    let bond_amount = provider.get_storage_at(kuda_address, U256::from(5)).await?;
    kuda_instance
        .submitOperatorBond()
        .value(bond_amount)
        .send()
        .await?
        .get_receipt()
        .await?;

    println!("Registering operator with KUDA");
    core_instance
        .registerOperatorToDSS(kuda_address, Bytes::new())
        .send()
        .await?
        .get_receipt()
        .await?;

    let stake_update_request = Operator::StakeUpdateRequest {
        vault: vault_address,
        dss: kuda_address,
        toStake: true,
    };
    println!("Requesting stake update");
    let queued_stake_update = core_instance
        .requestUpdateVaultStakeInDSS(stake_update_request.clone())
        .send()
        .await?
        .get_receipt()
        .await?
        .inner
        .logs()[1]
        .log_decode::<Core::RequestedStakeUpdate>()?
        .inner
        .updateRequest
        .clone();

    println!("Finalizing stake update");
    core_instance
        .finalizeUpdateVaultStakeInDSS(queued_stake_update)
        .send()
        .await?
        .get_receipt()
        .await?;

    Ok(())
}

// TODO: Refactor this into a separate module and reuse in operator binary
fn local_signer(keystore_path: &PathBuf) -> Result<PrivateKeySigner> {
    print!("Enter keystore passphrase: ");
    std::io::stdout().flush()?;

    let passphrase = rpassword::read_password()?;
    Ok(PrivateKeySigner::decrypt_keystore(
        keystore_path,
        passphrase,
    )?)
}

// TODO: Refactor this into a separate module and reuse in operator binary
async fn aws_signer(
    region: String,
    access_key_id: &str,
    secret_access_key: &str,
    operator_key_id: String,
) -> Result<AwsSigner> {
    let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .credentials_provider(SharedCredentialsProvider::new(credentials))
        .load()
        .await;

    let client = aws_sdk_kms::Client::new(&aws_config);
    let signer = AwsSigner::new(client.clone(), operator_key_id, None).await?;

    Ok(signer)
}
