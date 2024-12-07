use std::{cmp, sync::Arc};

use alloy::{
    consensus::{utils::WholeFe, Bytes48, SidecarBuilder, SidecarCoder},
    network::{Ethereum, EthereumWallet, TransactionBuilder, TransactionBuilder4844, TxSigner},
    primitives::{Address, B256},
    providers::{
        fillers::{
            BlobGasFiller, CachedNonceManager, ChainIdFiller, FillProvider, GasFiller, JoinFill,
            NonceFiller, WalletFiller,
        },
        Identity, Provider, ProviderBuilder, ReqwestProvider,
    },
    rpc::types::{BlockTransactionsKind, TransactionRequest},
    transports::http::ReqwestTransport,
};
use eyre::OptionExt;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use url::Url;

use crate::kms::KmsSigner;

use super::{BlobData, Submitter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eip4844Receipt {
    pub beacon_block_slot: u64,
    pub commitment: Bytes48,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TerminationCoder(pub usize);

impl TerminationCoder {
    /// Decode an some bytes from an iterator of valid FEs.
    ///
    /// Returns `Ok(Some(data))` if there is some data.
    /// Returns `Ok(None)` if there is no data (length prefix is 0).
    /// Returns `Err(_)` if there is an error.
    fn decode_one<'a>(fes: impl Iterator<Item = WholeFe<'a>>) -> eyre::Result<Option<Vec<u8>>> {
        let mut res = Vec::new();
        let regex = regex::bytes::Regex::new(r"(?-u)\x80[\x00]*")?;

        // This is very inefficient, but we won't be using decoding primarily
        // so it should be fine
        for fe in fes {
            let bytes = &fe.as_ref()[1..32];
            if let Some(mid) = regex.find(bytes).map(|mid| mid.start()) {
                let to_append = bytes.split_at(mid).0;
                res.extend_from_slice(to_append);
                break;
            } else {
                tracing::warn!("No match");
                res.extend_from_slice(bytes);
            }
        }
        Ok(Some(res))
    }
}

impl SidecarCoder for TerminationCoder {
    fn required_fe(&self, data: &[u8]) -> usize {
        data.len().div_ceil(31)
    }

    fn code(
        &mut self,
        builder: &mut alloy::eips::eip4844::builder::PartialSidecar,
        mut data: &[u8],
    ) {
        // ingest the rest of the data
        loop {
            let mid = cmp::min(31, data.len());
            let (left, right) = data.split_at(mid);
            self.0 += mid;
            // If we have less than 31 bytes, we can stop processing
            // and push a terminator byte to indicate the end of the blob.
            if left.len() < 31 {
                let mut to_ingest = left.to_vec();
                to_ingest.push(0x80);
                builder.ingest_partial_fe(&to_ingest);
                return;
            } else {
                builder.ingest_partial_fe(left);
            }
            data = right
        }
    }

    fn decode_all(&mut self, blobs: &[alloy::consensus::Blob]) -> Option<Vec<Vec<u8>>> {
        let mut fes = blobs
            .iter()
            .flat_map(|blob| blob.chunks(32).map(WholeFe::new))
            .map(Option::unwrap);
        let mut res = Vec::new();
        let mut decoded_length = 0;
        loop {
            match Self::decode_one(&mut fes) {
                Ok(Some(data)) => {
                    decoded_length += data.len();
                    res.push(data);
                    if decoded_length >= self.0 {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => return None,
            }
        }
        tracing::info!(
            "Coded length: {}, decoded length: {}",
            self.0,
            decoded_length
        );
        Some(res)
    }

    fn finish(self, _: &mut alloy::eips::eip4844::builder::PartialSidecar) {}
}

/// The response to a request for a __single__ beacon block: `blocks/{id}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockResponse {
    /// Container for the header data.
    pub data: BlockData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockData {
    pub message: BeaconBlock,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlock {
    /// The slot to which this block corresponds.
    #[serde_as(as = "DisplayFromStr")]
    pub slot: u64,
    /// The tree hash Merkle root of the BeaconBlockBody for the BeaconBlock
    pub body: BeaconBlockBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockBody {
    pub execution_payload: ExecutionPayload,
}

#[serde_as]
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPayload {
    #[serde_as(as = "DisplayFromStr")]
    pub block_number: String,
    #[serde_as(as = "DisplayFromStr")]
    pub timestamp: u64,
    /// The block hash of the execution payload.
    pub block_hash: B256,
}

type RecommendedProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<
                GasFiller,
                JoinFill<BlobGasFiller, JoinFill<NonceFiller<CachedNonceManager>, ChainIdFiller>>,
            >,
        >,
        WalletFiller<EthereumWallet>,
    >,
    ReqwestProvider,
    ReqwestTransport,
    Ethereum,
>;

pub struct Eip4844Client {
    from: Address,
    to: Address,
    provider: RecommendedProvider,
    beacon_url: Url,
    reqwest_client: reqwest::Client,
}

impl Eip4844Client {
    pub fn new(
        signer: Arc<dyn KmsSigner + Send + Sync + 'static>,
        to: Address,
        rpc_url: Url,
        beacon_url: Url,
    ) -> eyre::Result<Self> {
        let filler = JoinFill::new(
            GasFiller,
            JoinFill::new(
                BlobGasFiller,
                JoinFill::new(
                    NonceFiller::new(CachedNonceManager::default()),
                    ChainIdFiller::default(),
                ),
            ),
        );
        let from = signer.address();
        let provider = ProviderBuilder::new()
            .filler(filler)
            .wallet(EthereumWallet::from(signer))
            .on_http(rpc_url);
        Ok(Self {
            from,
            to,
            provider,
            beacon_url,
            reqwest_client: reqwest::Client::new(),
        })
    }
}

impl Submitter for Eip4844Client {
    type Receipt = Eip4844Receipt;

    async fn submit(
        &self,
        provided_commitment: &[u8],
        blob_data: BlobData,
    ) -> eyre::Result<Self::Receipt> {
        // Create a sidecar with some data.
        let sidecar: SidecarBuilder<TerminationCoder> = SidecarBuilder::from_slice(&blob_data.data);
        let sidecar = sidecar.build()?;
        let commitment = sidecar.commitments[0];
        if provided_commitment != commitment {
            return Err(eyre::eyre!(
                "Provided commitment does not match computed commitment {} != {}",
                hex::encode(provided_commitment),
                commitment.to_string()
            ));
        }

        let tx = TransactionRequest::default()
            .with_from(self.from)
            .with_to(self.to)
            .with_blob_sidecar(sidecar);

        // Send the transaction and wait for the receipt.
        let receipt = self
            .provider
            .send_transaction(tx)
            .await?
            .get_receipt()
            .await?;

        let block_hash = receipt.block_hash.ok_or_eyre("No block hash in receipt")?;
        let block = self
            .provider
            .get_block_by_hash(block_hash, BlockTransactionsKind::Hashes)
            .await?
            .ok_or_eyre("Could not get block")?;

        let block_timestamp = block.header.timestamp;
        let parent_beacon_block_root = block
            .header
            .parent_beacon_block_root
            .ok_or_eyre("No parent beacon block root")?;

        let parent_beacon_block = self
            .reqwest_client
            .get(
                self.beacon_url
                    .join("eth/v2/beacon/blocks/")?
                    .join(&parent_beacon_block_root.to_string())?,
            )
            .send()
            .await?
            .json::<BlockResponse>()
            .await?;

        let parent_beacon_block_timestamp = parent_beacon_block
            .data
            .message
            .body
            .execution_payload
            .timestamp;

        // Calculate the slot difference between the parent beacon block and the current block.
        let slot_difference = (block_timestamp - parent_beacon_block_timestamp) / 12;

        let beacon_block_slot = parent_beacon_block.data.message.slot + slot_difference;

        let tx_hash = receipt.transaction_hash;
        let mut msg =
            format!("[EIP4844] Submitted blob with commitment {commitment} with transaction hash: {tx_hash}");
        if let Some(block_number) = receipt.block_number {
            msg.push_str(&format!(
                " at block number: {block_number} and slot: {beacon_block_slot}"
            ));
        }
        tracing::info!("{msg}");

        let receipt = Eip4844Receipt {
            beacon_block_slot,
            commitment,
        };
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_termination_coder() {
        let data = b"hello world".to_vec();

        let mut builder = alloy::eips::eip4844::builder::PartialSidecar::default();
        let mut coder = TerminationCoder::default();
        coder.code(&mut builder, &data);
        coder.finish(&mut builder);
        assert_eq!(coder.0, data.len());
        let blobs = builder.blobs();
        let decoded = coder.decode_all(blobs).unwrap();
        assert_eq!(decoded[0], data);

        let sidecar: SidecarBuilder<TerminationCoder> = SidecarBuilder::from_slice(&data);
        let blobs = sidecar.clone().take();
        let blob = blobs[0];
        let mut expected_blob = vec![0u8];
        expected_blob.extend_from_slice(&data);
        assert_eq!(&blob[..12], expected_blob);
        let commitment = sidecar.build().unwrap().commitments[0].to_string();
        assert_eq!(commitment, "0xb93ab7583ad8a57b2edd262889391f37a83ab41107dc02c1a68220841379ae828343e84ac1c70fb7c2640ee3522c4c36");
    }
}
