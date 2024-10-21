use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use axum::{routing::get, Router};
use clap::ValueEnum;
use serde::Deserialize;
use tokio::sync::RwLock;
use url::Url;

pub mod contracts;
pub mod da;
pub mod health;
pub mod operator;
pub mod socketio;

#[derive(Deserialize, Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SignerType {
    Local,
    Aws,
}

#[derive(Deserialize, Debug)]
pub struct EnvConfig {
    pub aggregator_url: Url,
    pub celestia_rpc_url: Url,
    pub celestia_auth_token: String,
    pub kuda_contract_address: Address,
    pub core_contract_address: Address,
    pub kuda_rpc_url: Url,

    pub signer_type: SignerType,

    pub aws_region: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,

    pub aws_operator_key_id: Option<String>,
    pub aws_eip4844_key_id: Option<String>,

    // Either keystore or keypair must be set when using local signer
    pub operator_keystore_path: Option<PathBuf>,
    pub operator_keystore_password: Option<String>,
    #[serde(
        deserialize_with = "deserialize_private_key",
        default = "default_private_key"
    )]
    pub operator_private_key: Option<PrivateKeySigner>,

    // Either keystore or keypair must be set when using local signer
    pub eip4844_keystore_path: Option<PathBuf>,
    pub eip4844_keystore_password: Option<String>,
    #[serde(
        deserialize_with = "deserialize_private_key",
        default = "default_private_key"
    )]
    pub eip4844_private_key: Option<PrivateKeySigner>,

    pub eip4844_to_address: Address,
    pub eip4844_rpc_url: Url,
    pub eip4844_beacon_url: Url,
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,

    pub otel_exporter_otlp_endpoint: Option<Url>,
}

fn default_host() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

fn default_port() -> u16 {
    8080
}

fn deserialize_private_key<'de, D>(deserializer: D) -> Result<Option<PrivateKeySigner>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <Option<String>>::deserialize(deserializer)?;
    Ok(s.as_deref()
        .map(PrivateKeySigner::from_str)
        .transpose()
        .ok()
        .flatten())
}

fn default_private_key() -> Option<PrivateKeySigner> {
    None
}

pub fn routes(socket_io_connected: Arc<RwLock<bool>>) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .with_state(socket_io_connected)
}
