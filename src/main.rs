use std::{net::IpAddr, path::PathBuf, sync::Arc};

use alloy::{
    network::{EthereumWallet, TxSigner},
    primitives::Address,
    providers::ProviderBuilder,
};
use clap::{
    builder::{styling::AnsiColor, Styles},
    Parser, Subcommand,
};
use kuda_operator::{
    contracts::kuda::Kuda::{self},
    da::{celestia::CelestiaClient, eip4844::Eip4844Client},
    operator::Operator,
    register::{register, RegisterConfig},
    run::{run, RunConfig},
    Kms,
};
use url::Url;

const CLAP_STYLING: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum KudaOperatorCommand {
    Run {
        #[arg(short, long, env)]
        aggregator_url: Url,

        #[arg(long, env)]
        celestia_rpc_url: Url,

        #[arg(long, env)]
        celestia_auth_token: Option<String>,

        #[arg(long, env, required_if_eq("kms", "aws"))]
        aws_eip4844_key_id: Option<String>,

        #[arg(long, env, required_if_eq("kms", "local"))]
        eip4844_keystore_path: Option<PathBuf>,

        #[arg(long, env, required_if_eq("kms", "local"))]
        eip4844_keystore_password: Option<String>,

        #[arg(long, env)]
        eip4844_to_address: Address,

        #[arg(long, env)]
        eip4844_rpc_url: Url,

        #[arg(long, env)]
        eip4844_beacon_url: Url,

        #[arg(long, env)]
        otel_exporter_otlp_endpoint: Option<Url>,

        #[arg(long, env, default_value = "0.0.0.0")]
        host: IpAddr,

        #[arg(long, env, default_value = "8080")]
        port: u16,
    },

    Register,
}

#[derive(Parser)]
#[command(version, about, long_about, styles = CLAP_STYLING)]
struct KudaOperator {
    #[command(subcommand)]
    command: KudaOperatorCommand,

    #[arg(short, long, env, default_value = "local", global = true)]
    kms: Kms,

    #[arg(long, env, required_if_eq("kms", "aws"), global = true)]
    aws_region: Option<String>,

    #[arg(long, env, required_if_eq("kms", "aws"), global = true)]
    aws_access_key_id: Option<String>,

    #[arg(long, env, required_if_eq("kms", "aws"), global = true)]
    aws_secret_access_key: Option<String>,

    #[arg(long, env, required_if_eq("kms", "aws"), global = true)]
    aws_operator_key_id: Option<String>,

    #[arg(long, env, required_if_eq("kms", "local"), global = true)]
    operator_keystore_path: Option<PathBuf>,

    #[arg(long, env, required_if_eq("kms", "local"), global = true)]
    operator_keystore_password: Option<String>,

    #[arg(
        long,
        env,
        default_value = "0x0000000000000000000000000000000000000000",
        global = true
    )]
    kuda_contract_address: Address,

    #[arg(
        long,
        env,
        default_value = "0x0000000000000000000000000000000000000000",
        global = true
    )]
    core_contract_address: Address,

    #[arg(long, env, default_value = "http://localhost:8545", global = true)]
    kuda_rpc_url: Url,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let cli = KudaOperator::parse();

    let operator_kms = match cli.kms {
        Kms::Local => {
            let operator_keystore_path = cli
                .operator_keystore_path
                .expect("Operator keystore path must be set when using local signer and keystore");
            let passphrase = match cli.operator_keystore_password {
                Some(password) => password,
                None => rpassword::prompt_password("Enter passphrase:")?,
            };
            kuda_operator::kms::Kms::Local {
                keystore: operator_keystore_path,
                passphrase,
            }
        }
        Kms::Aws => {
            let aws_region = cli
                .aws_region
                .clone()
                .expect("AWS region must be set when using AWS signer");
            let aws_access_key_id = cli
                .aws_access_key_id
                .clone()
                .expect("AWS access key ID must be set when using AWS signer");
            let aws_secret_access_key = cli
                .aws_secret_access_key
                .clone()
                .expect("AWS secret access key must be set when using AWS signer");
            let aws_operator_key_id = cli
                .aws_operator_key_id
                .clone()
                .expect("Operator private key ID must be set when using AWS signer");
            kuda_operator::kms::Kms::Aws {
                region: aws_region,
                access_key_id: aws_access_key_id,
                secret_access_key: aws_secret_access_key,
                key_id: aws_operator_key_id,
            }
        }
    };

    let operator_signer = kuda_operator::kms::get_signer(operator_kms).await?;
    let operator_address = operator_signer.address();
    let operator_wallet = EthereumWallet::from(operator_signer.clone());

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(operator_wallet)
        .on_http(cli.kuda_rpc_url);

    if cli.kuda_contract_address == Address::default() {
        tracing::error!("KUDA contract address must be set");
        return Err(eyre::eyre!("KUDA contract address must be set"));
    }
    if cli.core_contract_address == Address::default() {
        tracing::error!("Core contract address must be set");
        return Err(eyre::eyre!("Core contract address must be set"));
    }
    let kuda_instance = Arc::new(Kuda::new(cli.kuda_contract_address, provider.clone()));

    let operator = Arc::new(Operator::new(
        operator_address,
        cli.kuda_contract_address,
        cli.core_contract_address,
        provider.clone(),
    ));

    match cli.command {
        KudaOperatorCommand::Run {
            aggregator_url,
            celestia_rpc_url,
            celestia_auth_token,
            aws_eip4844_key_id,
            eip4844_keystore_path,
            eip4844_keystore_password,
            eip4844_to_address,
            eip4844_rpc_url,
            eip4844_beacon_url,
            otel_exporter_otlp_endpoint,
            host,
            port,
        } => {
            let eip4844_kms = match cli.kms {
                Kms::Local => {
                    let eip4844_keystore_path = eip4844_keystore_path.expect(
                        "EIP-4844 keystore path must be set when using local signer and keystore",
                    );
                    let passphrase = match eip4844_keystore_password {
                        Some(password) => password,
                        None => rpassword::prompt_password("Enter passphrase:")?,
                    };
                    kuda_operator::kms::Kms::Local {
                        keystore: eip4844_keystore_path,
                        passphrase,
                    }
                }
                Kms::Aws => {
                    let aws_region = cli
                        .aws_region
                        .expect("AWS region must be set when using AWS signer");
                    let aws_access_key_id = cli
                        .aws_access_key_id
                        .expect("AWS access key ID must be set when using AWS signer");
                    let aws_secret_access_key = cli
                        .aws_secret_access_key
                        .expect("AWS secret access key must be set when using AWS signer");
                    let aws_eip4844_key_id = aws_eip4844_key_id
                        .expect("EIP-4844 private key ID must be set when using AWS signer");
                    kuda_operator::kms::Kms::Aws {
                        region: aws_region,
                        access_key_id: aws_access_key_id,
                        secret_access_key: aws_secret_access_key,
                        key_id: aws_eip4844_key_id,
                    }
                }
            };
            let eip4844_signer = kuda_operator::kms::get_signer(eip4844_kms).await?;
            let eip4844_client = Arc::new(Eip4844Client::new(
                eip4844_signer,
                eip4844_to_address,
                eip4844_rpc_url,
                eip4844_beacon_url,
            )?);
            let celestia_client = Arc::new(
                CelestiaClient::new(&celestia_rpc_url, celestia_auth_token.as_deref()).await?,
            );

            let config = RunConfig {
                aggregator_url,
                operator_signer,
                kuda_instance,
                operator,
                celestia_client,
                eip4844_client,
                otel_exporter_otlp_endpoint,
                host,
                port,
            };

            run(config).await?;
        }
        KudaOperatorCommand::Register => {
            let config = RegisterConfig {
                operator,
                kuda_address: cli.kuda_contract_address,
                operator_address,
                provider,
            };

            register(config).await?;
        }
    }

    Ok(())
}
