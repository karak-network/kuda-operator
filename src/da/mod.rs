use std::str::FromStr;

use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};

pub mod celestia;
pub mod eip4844;

#[allow(async_fn_in_trait)]
pub trait Submitter {
    type Receipt;

    async fn submit(
        &self,
        provided_commitment: &str,
        data: BlobData,
    ) -> eyre::Result<Self::Receipt>;
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct BlobData {
    pub namespace: Option<celestia::Namespace>,
    pub data: Vec<u8>,
}

impl FromStr for BlobData {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(s)?;
        let task_input = BlobData::try_from_slice(&bytes)?;
        Ok(task_input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_data_deserialization() {
        let data = BlobData {
            namespace: None,
            data: b"hello world".to_vec(),
        };
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(borsh::to_vec(&data).unwrap());
        println!("{}", encoded);
        let decoded = BlobData::from_str(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
