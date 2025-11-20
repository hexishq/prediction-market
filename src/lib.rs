use bytemuck::{Pod, Zeroable};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    instruction::Instruction,
    msg,
    program_error::ProgramError,
    pubkey::{find_program_address, Pubkey},
    sysvars::rent::Rent,
    ProgramResult,
};
mod token_account;
use pinocchio_system::instructions::Transfer;
use token_account::*;

entrypoint!(process_instruction);

const TOKEN_PROGRAM_2022: Pubkey = [
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218, 182, 26, 252, 77,
    131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
];

// Define the data structures for the program
#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Bet {
    pub creator: Pubkey,             // 0-32
    pub gamble_token_a_mint: Pubkey, // 32-64
    pub gamble_token_b_mint: Pubkey, // 64 -96
    pub gamble_vault_a: Pubkey,      // 96-128
    pub gamble_vault_b: Pubkey,      // 128-160
    pub total_amount: u64,           // 160-168
    pub winner: u8,                  // 168-169
    pub padding: [u8; 7],            // 169-176
}

pub enum BetInstruction {
    CreatePrediction { amount: u64 }, // admin command
    EndPrediction { winner: u8 },     // admin command
    PlaceBet { option: u8, amount: u64 },
    Claim {},
}

impl BetInstruction {
    // Unpack the instruction data
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::CreatePrediction { amount }
            }
            1 => {
                let option = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
                let amount = rest
                    .get(1..9)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::PlaceBet {
                    option: *option,
                    amount,
                }
            }
            2 => {
                let winner = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
                Self::EndPrediction { winner: *winner }
            }
            3 => Self::Claim {},
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = BetInstruction::unpack(instruction_data)?;

    match instruction {
        BetInstruction::CreatePrediction { amount } => {
            msg!("Instruction: CreateBet");
            create_bet(program_id, accounts, amount)
        }
        BetInstruction::PlaceBet { option, amount } => {
            msg!("Instruction: PlaceBet");
            place_bet(program_id, accounts, option, amount)
        }
        BetInstruction::EndPrediction { winner } => {
            msg!("Instruction: SettleBet");
            settle_bet(program_id, accounts, winner)
        }
        BetInstruction::Claim {} => {
            msg!("Instruction: Withdraw");
            claim(accounts)
        }
    }
}

// Create a new bet
fn create_bet(program_id: &Pubkey, accounts: &[AccountInfo], _amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let vault_a_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let vault_b_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_a_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_b_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_a = find_program_address(&[bet_account, 1], program_id);

    let mint_b = find_program_address(&[bet_account, 2], program_id);

    if mint_a_account.key() != &mint_a.0 {
        msg!("Mint A account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    if mint_b_account.key() != &mint_b.0 {
        msg!("Mint B account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;

    msg!("Creating bet account");

    // Create the bet account
    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: bet_account,
        lamports: rent.minimum_balance(std::mem::size_of::<Bet>()),
        space: std::mem::size_of::<Bet>() as u64,
        owner: program_id,
    }
    .invoke()?;

    // Create pool vaults
    pinocchio_associated_token_account::instructions::Create {
        funding_account: creator_account,
        account: vault_a_account,
        wallet: bet_account,
        mint: mint_a,
        system_program: todo!(),
        token_program: todo!(),
    }
    .invoke()?;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: todo!(),
        account: todo!(),
        wallet: todo!(),
        mint: todo!(),
        system_program: todo!(),
        token_program: todo!(),
    }
    .invoke()?;

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    bet.creator = *creator_account.key();
    bet.total_amount = 0;
    bet.winner = 0;

    Ok(())
}

fn initialize_mint(
    decimals: u8,
    mint_account: &AccountInfo,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
) -> ProgramResult {
    let instruction_data = vec![];
    Instruction {
        program_id: TOKEN_PROGRAM_2022,
        data: TokenInstruction::InitializeMint {
            decimals,
            mint_authority,
            freeze_authority,
        }
        .pack(),
        accounts: vec![
            AccountMeta::new(*mint_account.key(), false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    }
}

// Place a bet on one of the alternatives
fn place_bet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    option: u8,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let gambler_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if unsafe { *bet_account.owner() } != *program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    match option {
        1 => bet.option_a_votes += 1,
        2 => bet.option_b_votes += 1,
        _ => {
            msg!("Invalid option");
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    // Transfer lamports from gambler to bet account
    pinocchio_system::instructions::Transfer {
        from: gambler_account,
        to: bet_account,
        lamports: amount,
    }
    .invoke()?;

    bet.total_amount += amount;

    Ok(())
}

// Settle the bet and distribute the funds
fn settle_bet(program_id: &Pubkey, accounts: &[AccountInfo], winner: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if unsafe { *bet_account.owner() } != *program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    if *creator_account.key() != bet.creator {
        msg!("Only the creator can settle the bet");
        return Err(ProgramError::IllegalOwner);
    }

    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    bet.winner = winner;

    let total_amount = bet.total_amount;

    // Transfer the funds to the winners
    for account in accounts_iter {
        let share = (total_amount as f64 / winner_votes as f64) as u64;
        *account.try_borrow_mut_lamports()? += share;
    }

    *creator_account.try_borrow_mut_lamports()? += bet_account.lamports();

    bet.creator = Pubkey::default();
    bet.total_amount = 0;

    Ok(())
}

// [signer, signer_token_account, pool, vault_wsol, vault_token_a, vault_token_b, token_program]
fn claim(accounts: &[AccountInfo]) -> ProgramResult {
    let signer = accounts.get(0).ok_or(ProgramError::InvalidAccountData)?;
    let signer_token_account = accounts.get(1).ok_or(ProgramError::InvalidAccountData)?;
    let token_mint = accounts.get(2).ok_or(ProgramError::InvalidAccountData)?;
    let pool = accounts.get(3).ok_or(ProgramError::InvalidAccountData)?;
    let vault_wsol = accounts.get(4).ok_or(ProgramError::InvalidAccountData)?;
    let vault_token_a = accounts.get(5).ok_or(ProgramError::InvalidAccountData)?;
    let vault_token_b = accounts.get(6).ok_or(ProgramError::InvalidAccountData)?;
    let token_program = accounts.get(7).ok_or(ProgramError::InvalidAccountData)?;

    let token_mint = AtaAccessor::get_mint(signer_token_account.borrow_data_unchecked());

    let winner: u8 = unsafe { bytemuck::from_bytes(&pool.borrow_data_unchecked()[168]) };
    let (win_mint, to) = if winner == 1 {
        (
            AtaAccessor::get_mint(vault_token_a.borrow_data_unchecked()),
            vault_token_a,
        )
    } else if winner == 2 {
        (
            AtaAccessor::get_mint(vault_token_b.borrow_data_unchecked()),
            vault_token_b,
        )
    } else {
        return Err(ProgramError::InvalidAccountData);
    };

    let winner_amount = AtaAccessor::get_amount(&signer_token_account);

    if win_mint != token_mint {
        return Err(ProgramError::InvalidAccountData);
    }

    let signer_to_pool = Transfer {
        from: signer,
        to,
        lamports: winner_amount,
    };

    let pool_to_signer = Transfer {
        from: vault_wsol,
        to: signer,
        lamports: winner_amount,
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use litesvm::LiteSVM;
    use solana_account::Account;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::{Keypair, Signer};
    use solana_pubkey::Pubkey;
    use solana_transaction::Transaction;

    struct TestSetup {
        pub lite_svm: LiteSVM,
        pub our_program_id: Pubkey,
    }

    fn setup() -> TestSetup {
        const DEFAULT_PROGRAMS: [Pubkey; 0] = [];
        const DEFAULT_PROGRAMS_PATH: &str = "programs/";
        const OUR_PROGRAM_ID: Pubkey =
            Pubkey::from_str_const("EFPnDWAebC2S6CdqWC9gj9vcxHv3Hiba6zQfv2LxZKfR");
        const OUR_PROGRAM_PATH: &str = "../target/deploy/solana_gamble_onchain.so";

        let mut lite_svm = LiteSVM::new().with_default_programs().with_sysvars();

        lite_svm
            .add_program_from_file(OUR_PROGRAM_ID, OUR_PROGRAM_PATH)
            .expect("Failed to load our program!");

        // Load additional programs if any
        for program in DEFAULT_PROGRAMS.iter() {
            if let Err(e) = lite_svm
                .add_program_from_file(program, format!("{}/{}.so", DEFAULT_PROGRAMS_PATH, program))
            {
                eprintln!("Failed to load program {}: {}", program, e);
            }
        }

        TestSetup {
            lite_svm,
            our_program_id: OUR_PROGRAM_ID,
        }
    }

    #[test]
    fn test_create_bet() {
        let mut setup = setup();

        let creator_account = Keypair::new();

        setup
            .lite_svm
            .set_account(creator_account.pubkey(), Account::default())
            .expect("Failed to set creator account");

        setup
            .lite_svm
            .airdrop(&creator_account.pubkey(), 10_u64.pow(9))
            .expect("Failed to airdrop creator account  ");

        let bet_account = Keypair::new();

        // 0 = CreateBet instruction
        let mut instruction_data = vec![0];

        for byte in 1000_u64.to_le_bytes() {
            instruction_data.push(byte);
        }

        let accounts = vec![
            AccountMeta::new(creator_account.pubkey(), true),
            AccountMeta::new(bet_account.pubkey(), false),
        ];

        let instruction = Instruction {
            program_id: setup.our_program_id,
            accounts,
            data: instruction_data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&creator_account.pubkey()),
            &[&creator_account],
            setup.lite_svm.latest_blockhash(),
        );

        match setup.lite_svm.simulate_transaction(transaction) {
            Ok(res) => println!("Transaction succeeded with result: {:?}", res),
            Err(e) => panic!("Transaction failed: {}", e.err),
        }
    }
}
