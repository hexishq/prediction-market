use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bet {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub amount: f64,
    pub creator: String,
    pub pubkey: String, // Endereço da conta Solana onde está o dinheiro
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBetRequest {
    pub title: String,
    pub amount: f64,
    pub creator: String,
    pub pubkey: String,
}
