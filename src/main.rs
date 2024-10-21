use std::{net::SocketAddr, sync::Arc};

use alloy::{
    network::{EthereumWallet, TxSigner},
    providers::ProviderBuilder,
    signers::{aws::AwsSigner, local::PrivateKeySigner, Signature, Signer},
};
use aws_config::{BehaviorVersion, Region};
use clap::Command;
use kuda_operator::{
    contracts::kuda::Kuda,
    da::{celestia::CelestiaClient, eip4844::Eip4844Client},
    operator::Operator,
    socketio::socket_io,
    EnvConfig,
};
use metrics::describe_counter;
use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::{net::TcpListener, signal};
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    Command::new("kuda-operator")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KUDA Operator")
        .after_long_help(
            "You'll need to set the following environment variables:\n\
            - AGGREGATOR_URL\n\n\
            - CELESTIA_RPC_URL\n\
            - CELESTIA_AUTH_TOKEN\n\n\
            - KUDA_CONTRACT_ADDRESS\n\
            - CORE_CONTRACT_ADDRESS\n\
            - KUDA_RPC_URL\n\n\
            - SIGNER_TYPE [possible values: local, aws]\n\n\
            - AWS_REGION [required if SIGNER_TYPE=aws]\n\
            - AWS_ACCESS_KEY_ID [required if SIGNER_TYPE=aws]\n\
            - AWS_SECRET_ACCESS_KEY [required if SIGNER_TYPE=aws]\n\
            - AWS_OPERATOR_KEY_ID [required if SIGNER_TYPE=aws]\n\
            - AWS_EIP4844_KEY_ID [required if SIGNER_TYPE=aws]\n\n\
            - OPERATOR_KEYSTORE_PATH [required if SIGNER_TYPE=local]\n\
            - OPERATOR_KEYSTORE_PASSWORD [required if SIGNER_TYPE=local]\n\
            - EIP4844_KEYSTORE_PATH [required if SIGNER_TYPE=local]\n\
            - EIP4844_KEYSTORE_PASSWORD [required if SIGNER_TYPE=local]\n\n\
            - EIP4844_TO_ADDRESS\n\
            - EIP4844_RPC_URL\n\
            - EIP4844_BEACON_URL\n",
        )
        .get_matches();

    dotenvy::dotenv().ok();
    let config = envy::from_env::<EnvConfig>()?;

    // log level filtering here
    let filter_layer = EnvFilter::from_default_env();

    // fmt layer - printing out logs
    let fmt_layer = fmt::layer().compact();

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer);

    if let Some(otel_endpoint) = config.otel_exporter_otlp_endpoint {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(otel_endpoint),
            )
            .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
                Resource::new(vec![KeyValue::new(SERVICE_NAME, "kuda-operator")]),
            ))
            .install_batch(opentelemetry_sdk::runtime::Tokio)?
            .tracer("kuda-operator");

        // turn our OTLP pipeline into a tracing layer
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let subscriber = subscriber.with(otel_layer);
        subscriber.init();
    } else {
        subscriber.init();
    }

    PrometheusBuilder::new().install()?;
    describe_counter!(
        "posting_intent",
        "Counts the number of posting intents received"
    );
    describe_counter!(
        "task_responsibility",
        "Counts the number of posting intents assigned"
    );

    let (operator_wallet, operator_signer, eip_4844_signer): (
        EthereumWallet,
        Arc<dyn Signer + Send + Sync + 'static>,
        Arc<dyn TxSigner<Signature> + Send + Sync + 'static>,
    ) = match config.signer_type {
        kuda_operator::SignerType::Local => {
            let operator_signer = match config.operator_keystore_path {
                Some(operator_keystore_path) => {
                    let operator_keystore_password = config.operator_keystore_password.expect(
                            "Operator keystore password must be set when using local signer and keystore",
                        );

                    PrivateKeySigner::decrypt_keystore(
                        operator_keystore_path,
                        operator_keystore_password,
                    )?
                }
                None => config.operator_private_key.expect(
                    "Either operator keystore or keypair must be set when using local signer",
                ),
            };
            let operator_wallet = EthereumWallet::from(operator_signer.clone());

            let eip_4844_signer = match config.eip4844_keystore_path {
                Some(eip4844_keystore_path) => {
                    let eip4844_keystore_password = config
                        .eip4844_keystore_password
                        .expect("EIP-4844 keystore password must be set when using local signer and keystore");
                    PrivateKeySigner::decrypt_keystore(
                        eip4844_keystore_path,
                        eip4844_keystore_password,
                    )?
                }
                None => config.eip4844_private_key.expect(
                    "Either EIP-4844 keystore or keypair must be set when using local signer",
                ),
            };

            (
                operator_wallet,
                Arc::new(operator_signer),
                Arc::new(eip_4844_signer),
            )
        }
        kuda_operator::SignerType::Aws => {
            let region = config
                .aws_region
                .expect("AWS region must be set when using AWS signer");
            config
                .aws_access_key_id
                .expect("AWS access key ID must be set when using AWS signer");
            config
                .aws_secret_access_key
                .expect("AWS secret access key must be set when using AWS signer");
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region))
                .load()
                .await;
            let operator_private_key_id = config
                .aws_operator_key_id
                .expect("Operator private key ID must be set when using AWS signer");
            let eip4844_private_key_id = config
                .aws_eip4844_key_id
                .expect("EIP-4844 private key ID must be set when using AWS signer");
            let client = aws_sdk_kms::Client::new(&aws_config);
            let operator_signer =
                AwsSigner::new(client.clone(), operator_private_key_id, None).await?;
            let operator_wallet = EthereumWallet::from(operator_signer.clone());
            let eip4844_signer = AwsSigner::new(client, eip4844_private_key_id, None).await?;
            (
                operator_wallet,
                Arc::new(operator_signer),
                Arc::new(eip4844_signer),
            )
        }
    };
    let operator_address = operator_signer.address();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(operator_wallet)
        .on_http(config.kuda_rpc_url);
    let kuda_instance = Arc::new(Kuda::new(config.kuda_contract_address, provider.clone()));
    let eip4844_client = Arc::new(Eip4844Client::new(
        eip_4844_signer,
        config.eip4844_to_address,
        config.eip4844_rpc_url,
        config.eip4844_beacon_url,
    )?);
    let celestia_client = Arc::new(
        CelestiaClient::new(&config.celestia_rpc_url, Some(&config.celestia_auth_token)).await?,
    );

    let operator = Arc::new(Operator::new(
        operator_address,
        config.kuda_contract_address,
        config.core_contract_address,
        provider,
    ));

    if !operator.is_registered().await? {
        tracing::info!("Operator not registered with KUDA, registering");
        operator.register().await?;
    } else {
        let stake = operator.stake().await?;
        tracing::info!("Operator already registered with KUDA");
        tracing::info!("Stake: {:?}", stake);
    }

    let cancellation_token = CancellationToken::new();
    let socket_io_cancel = cancellation_token.clone();

    let socket_url = config.aggregator_url.clone();
    let is_connected = Arc::new(tokio::sync::RwLock::new(false));
    let is_connected_clone = is_connected.clone();
    let socket_io_task = tokio::spawn(async move {
        tokio::select! {
            biased;
            _ = async {
                while let Err(e) = socket_io(
                    socket_url.clone(),
                    celestia_client.clone(),
                    eip4844_client.clone(),
                    operator_signer.clone(),
                    kuda_instance.clone(),
                    socket_io_cancel.clone(),
                    is_connected_clone.clone(),
                )
                .await
                {
                    tracing::error!("Socket IO connection error: {e:?}");
                    *is_connected_clone.write().await = false;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } => {},
            _ = socket_io_cancel.cancelled() => {
                tracing::info!("Socket IO task cancelled");
            },
        }
    });

    let governor_config = Arc::new(GovernorConfig::default());
    let app = kuda_operator::routes(is_connected.clone())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_request(trace::DefaultOnRequest::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO))
                .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(ServiceBuilder::new().layer(GovernorLayer {
            config: governor_config.clone(),
        }));

    let listener = TcpListener::bind((config.host, config.port)).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    cancellation_token.cancel();
    let _ = socket_io_task.await;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down")
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down")
        },
    }
}
