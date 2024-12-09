# dice_game

# Dice Game(dice_game) - Solana Program (Smart Contract)

## Overview
Dice Game is a Solana-based blockchain gambling program implemented in Rust using the Anchor framework. This repository contains the core program logic within a single `lib.rs` file. The game enables players to pay to play, roll dice, score their rolls, and compete for a jackpot, with a leaderboard system for top scores. This project was developed using Solana Playground. 

## Features

### Core Game Mechanics:
- **Pay-to-play entry**: Players must pay an entry fee to participate.
- **Dice rolling and scoring**: Players roll a dice and score points based on the result.
- **Jackpot management**: The program manages the jackpot pool, with players competing for it.
- **Leaderboard tracking**: Displays top players based on scores.

### Optimized Dice Roll Storage:
- **Compressed representation**: The dice roll result is stored in a 16-bit integer to save space.

### Pseudo-Random Generation:
- **Dice rolls generated**: Dice rolls are pseudo-randomly generated via a hash of the player’s public key and a timestamp.

### Cooldown Mechanism:
- **Cooldown enforcement**: A 10-second cooldown is enforced between dice rolls for each player to prevent spamming.

### Program(Smart Contract) Functionality:
- **Custom error handling**: The contract includes specific error messages for various failure conditions.
- **Efficient state management**: The contract efficiently handles the state of the game, including scores, players, and jackpots.

## File Contents
This repository includes a single file:
- **`lib.rs`**: Contains the complete logic for the Dice Game smart contract, including all functions, account structures, and error handling.
-  client.ts and anchor.test.ts  are not in use for this project at the moment.

  ## License
  - This project is under the **MIT LICENSE**
