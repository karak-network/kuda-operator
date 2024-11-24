use std::sync::Arc;

use alloy::{
    primitives::{utils::format_units, Address},
    providers::Provider,
    transports::Transport,
};

use crate::{contracts::kuda::Kuda::KudaInstance, operator::Operator};

pub struct RegisterConfig<T: Transport + Clone, P: Provider<T> + Clone> {
    pub operator_address: Address,
    pub operator: Arc<Operator<T, P>>,
    pub kuda_instance: Arc<KudaInstance<T, P>>,
}

pub async fn register<T: Transport + Clone, P: Provider<T> + Clone>(
    config: RegisterConfig<T, P>,
) -> eyre::Result<()> {
    if config.operator.is_registered().await? {
        println!("Operator already registered with KUDA");
        return Ok(());
    }

    println!("Operator not registered with KUDA");
    println!("Checking operator bond");

    let min_operator_bond = config.kuda_instance.MIN_OPERATOR_BOND().call().await?._0;

    let operator_bond = config
        .kuda_instance
        .operatorBond(config.operator_address)
        .call()
        .await?
        .bond;

    let formatted_operator_bond = format!("{} ETH", format_units(operator_bond, "ether")?);
    let formatted_min_operator_bond = format!("{} ETH", format_units(min_operator_bond, "ether")?);

    println!("Operator bond = {formatted_operator_bond}");

    if operator_bond < min_operator_bond {
        let difference = min_operator_bond - operator_bond;
        let formatted_difference = format!("{} ETH", format_units(difference, "ether")?);
        println!("Operator bond = {formatted_operator_bond} is less than minimum operator bond = {formatted_min_operator_bond}\nSubmitting difference = {formatted_difference}");
        let tx_hash = config.operator.submit_operator_bond(difference).await?;
        println!("Operator bond submitted with tx hash: {tx_hash}");
    }

    let tx_hash = config.operator.register().await?;
    println!("Operator registered with KUDA with tx hash: {tx_hash}");

    Ok(())
}
