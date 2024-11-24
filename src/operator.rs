use std::{collections::HashMap, sync::Arc};

use alloy::{
    primitives::{utils::format_units, Address, Bytes, TxHash, U256},
    providers::Provider,
    transports::Transport,
};
use serde::Serialize;

use crate::contracts::{
    core::Core::CoreInstance, kuda::Kuda::KudaInstance, vault::Vault::VaultInstance,
};

#[derive(Debug, Serialize)]
pub struct Vault {
    pub symbol: String,
    pub name: String,
    #[serde(serialize_with = "serialize_u256")]
    pub amount: U256,
}

pub fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

pub struct Operator<T: Transport + Clone, P: Provider<T>> {
    pub operator_address: Address,
    pub kuda_address: Address,
    pub core_instance: Arc<CoreInstance<T, P>>,
    pub kuda_instance: Arc<KudaInstance<T, P>>,
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
        let kuda_instance = Arc::new(KudaInstance::new(kuda_address, provider.clone()));
        let provider = Arc::new(provider);
        Operator {
            operator_address,
            kuda_address,
            core_instance,
            kuda_instance,
            provider,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn submit_operator_bond(&self, amount: U256) -> eyre::Result<TxHash> {
        let balance = self.provider.get_balance(self.operator_address).await?;
        println!("Balance: {} ETH", format_units(balance, "ether")?);
        println!("Amount: {} ETH", format_units(amount, "ether")?);
        if balance < amount {
            return Err(eyre::eyre!("Insufficient balance"));
        }
        let receipt = self
            .kuda_instance
            .submitOperatorBond()
            .value(amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        tracing::info!(
            "Operator bond submitted with tx hash: {}",
            receipt.transaction_hash
        );
        Ok(receipt.transaction_hash)
    }

    #[tracing::instrument(skip(self))]
    pub async fn register(&self) -> eyre::Result<TxHash> {
        let receipt = self
            .core_instance
            .registerOperatorToDSS(self.kuda_address, Bytes::default())
            .send()
            .await?
            .get_receipt()
            .await?;
        tracing::info!(
            "Operator registered with tx hash: {}",
            receipt.transaction_hash
        );
        Ok(receipt.transaction_hash)
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
    pub async fn stake(&self) -> eyre::Result<HashMap<Address, Vault>> {
        let operator_vaults = self
            .core_instance
            .fetchVaultsStakedInDSS(self.operator_address, self.kuda_address)
            .call()
            .await?
            .vaults;
        let mut stake = HashMap::new();
        for vault_address in operator_vaults {
            let vault_instance = VaultInstance::new(vault_address, self.provider.clone());
            let symbol = vault_instance.symbol().call().await?._0;
            let name = vault_instance.name().call().await?._0;
            let amount = vault_instance.totalAssets().call().await?._0;
            let vault = Vault {
                symbol,
                name,
                amount,
            };
            stake.insert(vault_address, vault);
        }
        Ok(stake)
    }
}
