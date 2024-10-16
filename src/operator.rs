use std::{collections::HashMap, sync::Arc};

use alloy::{
    primitives::{Address, Bytes, U256},
    providers::Provider,
    transports::Transport,
};

use crate::contracts::{core::Core::CoreInstance, vault::Vault};

pub struct Operator<T: Transport + Clone, P: Provider<T>> {
    pub operator_address: Address,
    pub kuda_address: Address,
    pub core_instance: Arc<CoreInstance<T, P>>,
    pub provider: Arc<P>,
}

impl<T: Transport + Clone, P: Provider<T> + Clone> Operator<T, P> {
    pub fn new(
        operator_address: Address,
        kuda_address: Address,
        core_address: Address,
        provider: P,
    ) -> Self {
        let core_instance = Arc::new(CoreInstance::new(core_address, provider.clone()));
        let provider = Arc::new(provider);
        Operator {
            operator_address,
            kuda_address,
            core_instance,
            provider,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn register(&self) -> eyre::Result<()> {
        let receipt = self
            .core_instance
            .registerOperatorToDSS(self.kuda_address, Bytes::from_static(&[0u8]))
            .send()
            .await?
            .get_receipt()
            .await?;
        tracing::info!(
            "Operator registered with tx hash: {}",
            receipt.transaction_hash
        );
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn is_registered(&self) -> eyre::Result<bool> {
        let is_registered = self
            .core_instance
            .isOperatorRegisteredToDSS(self.operator_address, self.kuda_address)
            .call()
            .await?
            ._0;
        Ok(is_registered)
    }

    // TODO: Normalize to ETH
    #[tracing::instrument(skip(self))]
    pub async fn stake(&self) -> eyre::Result<HashMap<String, U256>> {
        let operator_vaults = self
            .core_instance
            .fetchVaultsStakedInDSS(self.operator_address, self.kuda_address)
            .call()
            .await?
            .vaults;
        let mut stake = HashMap::new();
        for vault in operator_vaults {
            let vault_instance = Vault::new(vault, self.provider.clone());
            let symbol = vault_instance.symbol().call().await?._0;
            stake.insert(symbol, vault_instance.totalAssets().call().await?._0);
        }
        Ok(stake)
    }
}
