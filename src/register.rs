use std::sync::Arc;

use alloy::{
    primitives::{keccak256, utils::format_units, Address, U256},
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
    pub min_operator_bond: U256,
}

pub async fn register<T: Transport + Clone, P: Provider<T> + Clone>(
    config: RegisterConfig<T, P>,
) -> eyre::Result<()> {
    if config.operator.is_registered().await? {
        println!("Operator already registered with KUDA");
    } else {
        println!("Operator not registered with KUDA");
        println!("Checking operator bond");

        let operator_bond_storage_slot = U256::from_be_slice(
            keccak256((config.operator_address, U256::from(9)).abi_encode()).as_ref(),
        );
        let operator_bond = config
            .provider
            .get_storage_at(config.kuda_address, operator_bond_storage_slot)
            .await?;

        let formatted_operator_bond = format!("{} ETH", format_units(operator_bond, "ether")?);
        let formatted_min_operator_bond =
            format!("{} ETH", format_units(config.min_operator_bond, "ether")?);

        println!("Operator bond = {formatted_operator_bond}");

        if operator_bond < config.min_operator_bond {
            let difference = config.min_operator_bond - operator_bond;
            let formatted_difference = format!("{} ETH", format_units(difference, "ether")?);
            println!("Operator bond = {formatted_operator_bond} is less than minimum operator bond = {formatted_min_operator_bond}\nSubmitting difference = {formatted_difference}");
            let tx_hash = config.operator.submit_operator_bond(difference).await?;
            println!("Operator bond submitted with tx hash: {tx_hash}");
        }

        let tx_hash = config.operator.register().await?;
        println!("Operator registered with KUDA with tx hash: {tx_hash}");
    }

    Ok(())
}
