use bytemuck::{Pod, Zeroable};

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Prediction {
    // Prediction creator (who created the bet), has authority to end it.
    pub creator: [u8; 32],
    // Tokens created for the pool, these are needed so we can know how much and if a user bet
    // on a determined side of the prediction.
    pub gamble_token_a_mint: [u8; 32],
    pub gamble_token_b_mint: [u8; 32],
    // Total amount of SOL deposited into the pool.
    pub total_amount: u64,
    // Which side won the prediction (0 = prediction active, 1 = Side 1 won, 2 = Side 2 won)
    pub winner: u8,
    // Padding to ensure alignment
    pub padding: [u8; 7],
}

/// Instructions used to interact with onchain program
pub enum PredictionInstruction {
    /// Creates a new prediction
    CreatePrediction {},
    /// Ends an existant prediction
    EndPrediction { winner: u8 },
    /// Bets on some side of the prediction
    PlaceBet { option: u8, amount: u64 },
    /// Claim SOL winnings after prediction has ended, if the user won
    Claim,
}
