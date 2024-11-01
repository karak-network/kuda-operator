use std::{path::PathBuf, sync::Arc};

use alloy::{
    network::TxSigner,
    signers::{aws::AwsSigner, local::PrivateKeySigner, Signer},
};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_kms::config::{Credentials, SharedCredentialsProvider};

pub enum Kms {
    Local {
        keystore: PathBuf,
        passphrase: String,
    },
    Aws {
        region: String,
        access_key_id: String,
        secret_access_key: String,
        key_id: String,
    },
}

pub trait KmsSigner: Signer + TxSigner<alloy::primitives::Signature> {}

impl KmsSigner for PrivateKeySigner {}

impl KmsSigner for AwsSigner {}

pub async fn get_signer(kms: Kms) -> eyre::Result<Arc<dyn KmsSigner + Send + Sync + 'static>> {
    match kms {
        Kms::Local {
            keystore,
            passphrase,
        } => {
            let signer = PrivateKeySigner::decrypt_keystore(keystore, passphrase)?;
            Ok(Arc::new(signer))
        }
        Kms::Aws {
            region,
            access_key_id,
            secret_access_key,
            key_id,
        } => {
            let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "");
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region))
                .credentials_provider(SharedCredentialsProvider::new(credentials))
                .load()
                .await;

            let client = aws_sdk_kms::Client::new(&aws_config);
            let signer = AwsSigner::new(client, key_id, None).await?;
            Ok(Arc::new(signer))
        }
    }
}
