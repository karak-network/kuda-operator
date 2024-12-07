use std::fmt::Display;

use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingIntent {
    pub task_id: Uuid,
    pub size: u64,
    pub client_address: Address,
    pub reward_amount: U256,
    pub reward_token: Address,
    pub acceptable_da_layers: Vec<DaLayer>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingInterest {
    pub task_id: Uuid,
    pub operator_address: Address,
    pub da_layer: DaLayer,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponsibility {
    pub task_id: Uuid,
    pub data: String,
    pub commitment: Bytes,
    pub da_layer: DaLayer,
    pub signature: String,
    pub submission_time: U256,
    pub client_address: Address,
    pub reward_token: Address,
    pub reward_amount: U256,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum DaLayer {
    #[serde(rename = "Celestia")]
    Celestia,
    #[serde(rename = "4844")]
    Eip4844,
}

impl From<DaLayer> for u8 {
    fn from(value: DaLayer) -> Self {
        match value {
            DaLayer::Celestia => 0,
            DaLayer::Eip4844 => 1,
        }
    }
}

impl Display for DaLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaLayer::Celestia => write!(f, "Celestia"),
            DaLayer::Eip4844 => write!(f, "4844"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ping {
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pong {
    pub id: Uuid,
    pub operator: Address,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json::json;

    #[test]
    fn test_posting_intent_deserialization() {
        let json = json!({
            "taskId": "f300e8d6-181c-4eb2-94b3-ff177ba6c685",
            "size": 100,
            "rewardAmount": "100",
            "rewardToken": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "clientAddress": "0x0000000000000000000000000000000000000000",
            "acceptableDaLayers": ["Celestia", "4844"]
        });
        let posting_intent = serde_json::from_value::<PostingIntent>(json).unwrap();
        assert_eq!(
            posting_intent.task_id,
            Uuid::from_str("f300e8d6-181c-4eb2-94b3-ff177ba6c685").unwrap()
        );
        assert_eq!(posting_intent.size, 100);
        assert_eq!(posting_intent.reward_amount, U256::from(100));
        assert_eq!(
            posting_intent.reward_token,
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap()
        );
        assert_eq!(posting_intent.client_address, Address::ZERO);
        assert_eq!(
            posting_intent.acceptable_da_layers,
            vec![DaLayer::Celestia, DaLayer::Eip4844]
        );
    }
}
