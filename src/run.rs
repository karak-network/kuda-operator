use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use alloy::{providers::Provider, transports::Transport};
use metrics::{describe_counter, describe_gauge, gauge};
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
use url::Url;

use crate::{
    contracts::kuda::Kuda::KudaInstance,
    da::{celestia::CelestiaClient, eip4844::Eip4844Client},
    kms::KmsSigner,
    operator::Operator,
    socketio::socket_io,
};

pub struct RunConfig<T: Transport + Clone, P: Provider<T>> {
    pub aggregator_url: Url,
    pub operator_signer: Arc<dyn KmsSigner + Send + Sync + 'static>,
    pub kuda_instance: Arc<KudaInstance<T, P>>,
    pub operator: Arc<Operator<T, P>>,
    pub celestia_client: Arc<CelestiaClient>,
    pub eip4844_client: Arc<Eip4844Client>,
    pub otel_exporter_otlp_endpoint: Option<Url>,
    pub host: IpAddr,
    pub port: u16,
}

pub async fn run<T: Transport + Clone, P: Provider<T> + Clone + 'static>(
    config: RunConfig<T, P>,
) -> eyre::Result<()> {
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
    describe_counter!(
        "task_responsibility_error",
        "Counts the number of assigned tasks that failed"
    );
    describe_counter!(
        "task_responsibility_success",
        "Counts the number of assigned tasks that succeeded"
    );
    describe_gauge!(
        "socket_io_connected",
        "Indicates if the socket io connection to the aggregator is established"
    );

    if !config.operator.is_registered().await? {
        tracing::error!("Operator not registered with KUDA, please register by running `kuda-operator register`");
        return Err(eyre::eyre!("Operator not registered with KUDA"));
    } else {
        let stake = config.operator.stake().await?;
        tracing::info!("Operator already registered with KUDA");
        tracing::info!("Stake: {}", serde_json::to_string_pretty(&stake)?);
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
                    config.celestia_client.clone(),
                    config.eip4844_client.clone(),
                    config.operator_signer.clone(),
                    config.kuda_instance.clone(),
                    socket_io_cancel.clone(),
                    is_connected_clone.clone(),
                )
                .await
                {
                    tracing::error!("Socket IO connection error: {e:?}");
                    *is_connected_clone.write().await = false;
                    gauge!("socket_io_connected").set(0);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } => {},
            _ = socket_io_cancel.cancelled() => {
                tracing::info!("Socket IO task cancelled");
            },
        }
    });

    let governor_config = Arc::new(GovernorConfig::default());
    let app = crate::routes(is_connected.clone())
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
