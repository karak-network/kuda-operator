use std::{str::FromStr, sync::Arc};

use alloy::{
    primitives::{Address, Bytes, FixedBytes},
    providers::Provider,
    signers::{Signature, Signer},
    sol_types::SolValue,
    transports::Transport,
};
use futures_util::FutureExt;
use model::{DaLayer, PostingIntent, PostingInterest, TaskResponsibility};
use rust_socketio::{asynchronous::ClientBuilder, Payload, TransportType};
use serde_json::json;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    contracts::kuda::Kuda::KudaInstance,
    da::{celestia::CelestiaClient, eip4844::Eip4844Client, BlobData, Submitter},
};

pub mod model;

pub async fn socket_io<T: Transport + Clone, P: Provider<T> + 'static>(
    socket_url: Url,
    celestia_client: Arc<CelestiaClient>,
    eip4844_client: Arc<Eip4844Client>,
    operator_signer: Arc<dyn Signer + Send + Sync + 'static>,
    kuda_instance: Arc<KudaInstance<T, P>>,
    cancellation_token: CancellationToken,
    is_connected: Arc<RwLock<bool>>,
) -> eyre::Result<()> {
    let operator_address = operator_signer.address();
    let message = "connection";
    let signature = hex::encode(
        operator_signer
            .sign_message(message.as_bytes())
            .await?
            .as_bytes(),
    );

    let connected_on_connect = is_connected.clone();
    let connected_on_disconnect = is_connected.clone();

    let kuda_instance_posting_intent = kuda_instance.clone();
    let builder = ClientBuilder::new(socket_url.clone())
        .transport_type(TransportType::Websocket)
        .namespace("/")
        .auth(json!({
            "signature": signature,
            "operatorAddress": operator_address,
        }))
        .reconnect_on_disconnect(true)
        .on(rust_socketio::Event::Connect, move |_, _| {
            let is_connected = connected_on_connect.clone();
            async move {
                *is_connected.write().await = true;
                tracing::info!("Connected to server");
            }
            .boxed()
        })
        .on(rust_socketio::Event::Close, move |_, _| {
            let is_connected = connected_on_disconnect.clone();
            async move {
                *is_connected.write().await = false;
                tracing::info!("Disconnected from server");
            }
            .boxed()
        })
        .on(rust_socketio::Event::Error, move |error, _| {
            async move {
                tracing::error!("Socket IO error: {error:?}");
            }
            .boxed()
        })
        .on("data-posting-intent", move |payload, client| {
            let operator_address = operator_address;
            let kuda_instance = kuda_instance_posting_intent.clone();
            async move {
                let result =
                    process_posting_intent(&payload, &client, &operator_address, &kuda_instance)
                        .await;
                if let Err(e) = result {
                    tracing::error!("Posting intent error: {e:?}");
                }
            }
            .boxed()
        })
        .on("task-responsibility", move |payload, _| {
            let celestia_client = celestia_client.clone();
            let eip4844_client = eip4844_client.clone();
            let kuda_instance = kuda_instance.clone();
            let operator_signer = operator_signer.clone();
            async move {
                let result = process_task_responsibility(
                    payload,
                    &celestia_client,
                    &eip4844_client,
                    operator_signer.clone(),
                    &kuda_instance,
                )
                .await;
                if let Err(e) = result {
                    tracing::error!("Task responsibility error: {e:?}");
                }
            }
            .boxed()
        });

    let client = builder.connect().await?;

    tokio::select! {
        _ = cancellation_token.cancelled() => {
            tracing::info!("Disconnecting socket io connection");
            let _ = client.disconnect().await;
        }
        _ = async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if !*is_connected.read().await {
                    break;
                }
            }
        } => {
            tracing::info!("Socket IO disconnected");
            return Err(eyre::eyre!("Socket IO disconnected"));
        }
    }

    Ok(())
}

#[tracing::instrument(skip(client, operator_address, kuda_instance))]
async fn process_posting_intent<T: Transport + Clone, P: Provider<T>>(
    payload: &Payload,
    client: &rust_socketio::asynchronous::Client,
    operator_address: &Address,
    kuda_instance: &KudaInstance<T, P>,
) -> eyre::Result<()> {
    if let Payload::Text(values) = payload {
        let posting_intent = serde_json::from_value::<PostingIntent>(values[0].clone())?;
        tracing::info!("Received task id: {}", posting_intent.task_id);
        // TODO: add custom logic to determine if we want to post data
        let client_balance = kuda_instance
            .kudaAccount(posting_intent.client_address, posting_intent.reward_token)
            .call()
            .await?
            .balance;
        if client_balance >= posting_intent.reward_amount {
            let posting_interest = PostingInterest {
                task_id: posting_intent.task_id,
                operator_address: *operator_address,
                da_layer: posting_intent.acceptable_da_layers[0],
            };
            client
                .emit(
                    "data-posting-interest",
                    serde_json::to_value(posting_interest)?,
                )
                .await?;
        } else {
            tracing::error!(
                "Client balance: {} is less than reward amount: {}",
                client_balance,
                posting_intent.reward_amount
            );
        }
    }
    Ok(())
}

#[tracing::instrument(skip(celestia_client, eip4844_client, operator_signer, kuda_instance))]
async fn process_task_responsibility<T: Transport + Clone, P: Provider<T>>(
    payload: Payload,
    celestia_client: &CelestiaClient,
    eip4844_client: &Eip4844Client,
    operator_signer: Arc<dyn Signer + Send + Sync + 'static>,
    kuda_instance: &KudaInstance<T, P>,
) -> eyre::Result<()> {
    if let Payload::Text(values) = payload {
        let task = serde_json::from_value::<TaskResponsibility>(values[0].clone())?;
        tracing::info!("Received task-responsibility: {}", task.task_id);
        let blob_data = BlobData::from_str(&task.data)?;
        let context = match task.da_layer {
            DaLayer::Celestia => {
                let receipt = celestia_client.submit(&task.commitment, blob_data).await?;
                Bytes::copy_from_slice(&(receipt.namespace.0, receipt.height).abi_encode())
            }
            DaLayer::Eip4844 => {
                let receipt = eip4844_client.submit(&task.commitment, blob_data).await?;
                Bytes::copy_from_slice(&receipt.beacon_block_slot.abi_encode())
            }
        };
        let signature = Signature::from_str(&task.signature)?;

        let receipt = kuda_instance
            .submitReceipt(
                operator_signer.address(),
                FixedBytes::from(task.task_id.as_bytes()),
                Bytes::copy_from_slice(&signature.as_bytes()),
                task.commitment.clone(),
                context,
                task.da_layer.into(),
                task.submission_time,
                task.client_address,
                task.reward_token,
                task.reward_amount,
            )
            .send()
            .await?
            .get_receipt()
            .await?;

        tracing::info!(
            "Submitted receipt with tx hash: {}",
            receipt.transaction_hash
        );
    }
    Ok(())
}
