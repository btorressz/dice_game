use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;

declare_id!("4sG1beLWFVJf69RS4xefnbFcuz8kQTgANrqQpFFb25cT");

#[program]
pub mod dice_game { 
    use super::*;

    // Initialize the game, including the PDA for the jackpot
    pub fn initialize_game(
        ctx: Context<InitializeGame>, 
        dev: Pubkey, 
        price_to_play: u64, 
        games_till_jackpot: u64
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        global_state.dev = dev;
        global_state.price_to_play = price_to_play;
        global_state.games_till_jackpot = games_till_jackpot;
        global_state.round = 1;
        global_state.current_jackpot = 0;
        global_state.highest_score = 0;
        global_state.current_winner = Pubkey::default();
        global_state.round_start_time = Clock::get()?.unix_timestamp;

        Ok(())
    }

    // Create a player-specific game state
    pub fn create_game_state(ctx: Context<CreateGameState>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        game_state.owner = ctx.accounts.player.key();
        game_state.credit = 0;
        game_state.can_roll = false;
        game_state.upper_score = 0;
        game_state.lower_score = 0;
        game_state.roll_dice_compressed = 0; // Initialize as 0
        game_state.last_roll_time = 0;

        Ok(())
    }

    // Pay to play and enter the game
    pub fn send_payment_to_play(ctx: Context<SendPaymentToPlay>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let global_state = &mut ctx.accounts.global_state;

        require!(game_state.credit == 0, CustomError::AlreadyInGame);
        require!(ctx.accounts.player.lamports() >= global_state.price_to_play, CustomError::InsufficientFunds);

        // Transfer payment to the jackpot PDA and dev
        let amount_to_dev = global_state.price_to_play / 4;
        let amount_to_jackpot = global_state.price_to_play - amount_to_dev;

        **ctx.accounts.jackpot_pda.to_account_info().try_borrow_mut_lamports()? += amount_to_jackpot;
        invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                ctx.accounts.player.key,
                &global_state.dev,
                amount_to_dev,
            ),
            &[
                ctx.accounts.player.to_account_info(),
                ctx.accounts.dev.to_account_info(),
            ],
        )?;

        // Update game state
        game_state.credit = 1;
        Ok(())
    }

    // Roll dice (using compressed representation)
pub fn roll_dice(ctx: Context<RollDice>) -> Result<()> {
    let game_state = &mut ctx.accounts.game_state;
    let clock = Clock::get()?;

    require!(game_state.credit == 1, CustomError::NotPaid);
    require!(!game_state.can_roll, CustomError::AlreadyRolled);
    require!(
        clock.unix_timestamp > game_state.last_roll_time + 10, 
        CustomError::CooldownActive
    );

    // Generate pseudo-random dice rolls (compressed)
    let seed = ctx.accounts.player.key().to_bytes();
    let timestamp_bytes = clock.unix_timestamp.to_be_bytes();
    
    // Concatenate seed and timestamp, then hash
    let mut combined = [0u8; 40]; // 32 (seed) + 8 (timestamp)
    combined[..32].copy_from_slice(&seed);
    combined[32..].copy_from_slice(&timestamp_bytes);
    
    let randomness = anchor_lang::solana_program::keccak::hash(&combined);
    
    let mut rolls: u16 = 0; // Compressed dice rolls
    for i in 0..5 {
        let roll = (randomness.to_bytes()[i] % 6) + 1;
        rolls |= (roll as u16) << (i * 3); // Pack into 3 bits per roll
    }

    // Save compressed rolls and mark as rolled
    game_state.roll_dice_compressed = rolls;
    game_state.can_roll = true;
    game_state.last_roll_time = clock.unix_timestamp;

    Ok(())
}
    // Score the dice roll
    pub fn score_roll(ctx: Context<ScoreRoll>, score_type: u8) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        require!(game_state.can_roll, CustomError::NotRolled);

        // Unpack dice rolls
        let dice = unpack_dice_rolls(game_state.roll_dice_compressed);
        let mut score = 0;

        match score_type {
            1 => score = dice.iter().filter(|&&x| x == 1).count() as u64 * 1000,
            2 => score = dice.iter().filter(|&&x| x == 2).count() as u64 * 2000,
            3 => score = dice.iter().filter(|&&x| x == 3).count() as u64 * 3000,
            4 => score = dice.iter().filter(|&&x| x == 4).count() as u64 * 4000,
            5 => score = dice.iter().filter(|&&x| x == 5).count() as u64 * 5000,
            6 => score = dice.iter().filter(|&&x| x == 6).count() as u64 * 6000,
            _ => return Err(CustomError::InvalidScoreType.into()),
        };

        // Update score and reset roll status
        game_state.upper_score += score;
        game_state.can_roll = false;

        Ok(())
    }

    // End the game and update the leaderboard
  pub fn game_over(ctx: Context<GameOver>) -> Result<()> {
          let game_state = &mut ctx.accounts.game_state;
          let global_state = &mut ctx.accounts.global_state;
           let leaderboard = &mut ctx.accounts.leaderboard;

        let final_score = game_state.upper_score + game_state.lower_score;

    // Update highest score and current winner
    if final_score > global_state.highest_score {
        global_state.highest_score = final_score;
        global_state.current_winner = ctx.accounts.player.key();
    }

    // Update leaderboard
    leaderboard.update(ctx.accounts.player.key(), final_score);

    // Reset player game state
    game_state.credit = 0;
    game_state.can_roll = false;
    game_state.upper_score = 0;
    game_state.lower_score = 0;
    game_state.roll_dice_compressed = 0;

    Ok(())
}

    // Withdraw the jackpot for the current winner
    pub fn withdraw_jackpot(ctx: Context<WithdrawJackpot>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        let jackpot_pda = &mut ctx.accounts.jackpot_pda;

        require!(global_state.current_winner == ctx.accounts.player.key(), CustomError::NotWinner);
        require!(**jackpot_pda.to_account_info().lamports.borrow() > 0, CustomError::NoJackpot);

        // Transfer jackpot to the winner
        let jackpot_amount = **jackpot_pda.to_account_info().lamports.borrow();
        **jackpot_pda.to_account_info().try_borrow_mut_lamports()? -= jackpot_amount;
        **ctx.accounts.player.to_account_info().try_borrow_mut_lamports()? += jackpot_amount;

        Ok(())
    }
}

// Helper functions
fn unpack_dice_rolls(packed: u16) -> Vec<u8> {
    let mut dice = Vec::new();
    for i in 0..5 {
        let roll = (packed >> (i * 3)) & 0x7; // Extract 3 bits per roll
        dice.push(roll as u8);
    }
    dice
}

// Accounts and Structs
#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(init, payer = payer, space = 8 + 200)] // Adjust space for GlobalState
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateGameState<'info> {
    #[account(init, payer = player, space = 8 + 100)] // Adjust space for GameState
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SendPaymentToPlay<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"jackpot".as_ref()],
        bump
    )]
    pub jackpot_pda: SystemAccount<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub dev: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RollDice<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ScoreRoll<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct GameOver<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub leaderboard: Account<'info, Leaderboard>,
    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawJackpot<'info> {
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        mut,
        seeds = [b"jackpot".as_ref()],
        bump
    )]
    pub jackpot_pda: SystemAccount<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
}

// State Accounts
#[account]
pub struct GlobalState {
    pub dev: Pubkey,
    pub current_jackpot: u64,
    pub highest_score: u64,
    pub current_winner: Pubkey,
    pub games_till_jackpot: u64,
    pub round_start_time: i64,
    pub round: u64,
    pub price_to_play: u64,
}

#[account]
pub struct GameState {
    pub owner: Pubkey,
    pub credit: u8,
    pub can_roll: bool,
    pub upper_score: u64,
    pub lower_score: u64,
    pub roll_dice_compressed: u16, // Compressed dice rolls
    pub last_roll_time: i64,
}

#[account]
pub struct Leaderboard {
    pub top_scores: [(Pubkey, u64); 10], // Top 10 scores with player addresses
}

impl Leaderboard {
    pub fn update(&mut self, player: Pubkey, score: u64) {
        for i in 0..self.top_scores.len() {
            if score > self.top_scores[i].1 {
                // Insert the new score and shift the rest
                self.top_scores.copy_within(i.., i + 1);
                self.top_scores[i] = (player, score);
                break;
            }
        }
    }
}

#[error_code]
pub enum CustomError {
    #[msg("Already in game.")]
    AlreadyInGame,
    #[msg("Insufficient funds.")]
    InsufficientFunds,
    #[msg("Player has not paid.")]
    NotPaid,
    #[msg("Dice already rolled.")]
    AlreadyRolled,
    #[msg("Player not rolled.")]
    NotRolled,
    #[msg("Invalid scoring type.")]
    InvalidScoreType,
    #[msg("No jackpot to withdraw.")]
    NoJackpot,
    #[msg("Not the winner.")]
    NotWinner,
    #[msg("Cooldown active.")]
    CooldownActive,
}
