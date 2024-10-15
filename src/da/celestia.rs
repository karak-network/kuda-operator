use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use celestia_rpc::{BlobClient, Client};
use celestia_types::{nmt::NS_SIZE, Blob, Commitment, TxConfig};
use url::Url;

use super::{BlobData, Submitter};

pub struct CelestiaReceipt {
    pub height: u64,
    pub commitment: Commitment,
    pub namespace: celestia_types::nmt::Namespace,
}

pub struct CelestiaClient {
    client: Client,
}

impl CelestiaClient {
    pub async fn new(url: &Url, token: Option<&str>) -> eyre::Result<Self> {
        let client = Client::new(url.as_str(), token).await?;
        Ok(Self { client })
    }
}

impl Submitter for CelestiaClient {
    type Receipt = CelestiaReceipt;

    async fn submit(
        &self,
        provided_commitment: &str,
        blob_data: BlobData,
    ) -> eyre::Result<Self::Receipt> {
        let blob = Blob::try_from(blob_data)?;
        let namespace = blob.namespace;
        let commitment = blob.commitment;
        let computed_commitment = base64::engine::general_purpose::STANDARD.encode(commitment.0);

        if provided_commitment != computed_commitment {
            return Err(eyre::eyre!(
                "Provided commitment does not match computed commitment"
            ));
        }

        // submit it
        let height = self
            .client
            .blob_submit(&[blob], TxConfig::default())
            .await?;

        tracing::info!(
            "[Celestia] Submitted blob with commitment {computed_commitment} at height {height} "
        );

        let receipt = CelestiaReceipt {
            height,
            commitment,
            namespace,
        };
        Ok(receipt)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Namespace(pub [u8; NS_SIZE]);

impl TryFrom<BlobData> for Blob {
    type Error = eyre::Error;

    fn try_from(blob_data: BlobData) -> Result<Self, Self::Error> {
        let Some(namespace) = blob_data.namespace else {
            return Err(eyre::eyre!(
                "Namespace is required for Celestia submissions"
            ));
        };

        let namespace = celestia_types::nmt::Namespace::new_v0(&namespace.0[1..])?;
        Ok(Blob::new(namespace, blob_data.data)?)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_celestia_input_from_str() {
        let namespace =
            celestia_types::nmt::Namespace::new_v0(&hex::decode("f2ac04b34d6f93be5323").unwrap())
                .unwrap();
        let blob_data = BlobData {
            namespace: Some(Namespace(namespace.0)),
            data: b"hello world".to_vec(),
        };
        let encoded = base64::prelude::BASE64_STANDARD.encode(borsh::to_vec(&blob_data).unwrap());
        let decoded = BlobData::from_str(&encoded).unwrap();
        assert_eq!(blob_data, decoded);
    }
}
