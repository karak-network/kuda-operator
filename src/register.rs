use std::sync::Arc;

use alloy::{
    primitives::{keccak256, Address, U256},
    providers::Provider,
    sol_types::SolValue,
    transports::Transport,
};

use crate::operator::Operator;

pub struct RegisterConfig<T: Transport + Clone, P: Provider<T>> {
    pub operator_address: Address,
    pub operator: Arc<Operator<T, P>>,
    pub provider: P,
    pub kuda_address: Address,
}

pub async fn register<T: Transport + Clone, P: Provider<T> + Clone>(
    config: RegisterConfig<T, P>,
) -> eyre::Result<()> {
    if config.operator.is_registered().await? {
        println!("Operator already registered with KUDA");
    } else {
        let operator_bond_storage_slot = U256::from_be_slice(
            keccak256((config.operator_address, U256::from(9)).abi_encode()).as_ref(),
        );
        let operator_bond = config
            .provider
            .get_storage_at(config.kuda_address, operator_bond_storage_slot)
            .await?;
        println!("Operator bond: {operator_bond}");

        let min_operator_bond = config
            .provider
            .get_storage_at(config.kuda_address, U256::from(5))
            .await?;

        if operator_bond < min_operator_bond {
            println!("Operator bond is less than minimum operator bond, submitting operator bond");
            let tx_hash = config
                .operator
                .submit_operator_bond(min_operator_bond - operator_bond)
                .await?;
            println!("Operator bond submitted with tx hash: {tx_hash}");
        }

        let tx_hash = config.operator.register().await?;
        println!("Operator registered with KUDA with tx hash: {tx_hash}");
    }

    Ok(())
}
