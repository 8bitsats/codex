use std::path::PathBuf;

use anchor_client::solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
};
use anchor_client::{Client, Cluster};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use coinflip::state::State;

/// CLI for interacting with the CoinFlip Solana program
#[derive(Parser)]
#[command(author, version, about = "CoinFlip CLI")]
struct Cli {
    /// RPC URL (e.g. Helius endpoint)
    #[arg(long, env = "HELIUS_RPC_URL")]
    rpc_url: String,
    /// Program ID of the deployed CoinFlip program
    #[arg(long, env = "COINFLIP_PROGRAM_ID")]
    program_id: Pubkey,
    /// Path to Solana JSON keypair for signing transactions
    #[arg(long, env = "SOLANA_KEYPAIR", value_parser)]
    keypair: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the CoinFlip program (create state and vault)
    Init { mint: Pubkey },
    /// Deposit tokens into the vault (only owner)
    Deposit { amount: u64 },
    /// Flip a coin by staking tokens on Heads or Tails
    Flip { amount: u64, side: Side },
    /// Withdraw tokens from the vault (only owner)
    Withdraw { amount: u64 },
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum Side {
    Heads,
    Tails,
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();
    let payer = read_keypair_file(&cli.keypair)?;
    let cluster = Cluster::Custom(cli.rpc_url.clone(), cli.rpc_url.clone());
    let client = Client::new_with_options(cluster, payer, CommitmentConfig::confirmed());
    let program = client.program(cli.program_id);

    let (state_pda, _state_bump) = Pubkey::find_program_address(&[b"state"], &cli.program_id);
    let (vault_pda, vault_bump) = Pubkey::find_program_address(&[b"vault"], &cli.program_id);

    match cli.command {
        Commands::Init { mint } => {
            let owner = program.payer().pubkey();
            let tx = program
                .request()
                .accounts(coinflip::accounts::Initialize {
                    state: state_pda,
                    vault: vault_pda,
                    vault_authority: vault_pda,
                    owner,
                    mint,
                    system_program: anchor_client::solana_sdk::system_program::ID,
                    token_program: anchor_spl::token::ID,
                    rent: anchor_client::solana_sdk::sysvar::rent::ID,
                })
                .args(coinflip::instruction::Initialize {})
                .send()?;
            println!("Initialized. State PDA: {}", state_pda);
        }

        Commands::Deposit { amount } => {
            let owner = program.payer().pubkey();
            let mint = {
                let info = program.rpc().get_account(&vault_pda)?;
                let account =
                    anchor_spl::token::TokenAccount::try_deserialize(&mut info.data.as_ref())?;
                account.mint
            };
            let owner_ata = get_associated_token_address(&owner, &mint);
            program
                .request()
                .accounts(coinflip::accounts::Deposit {
                    state: state_pda,
                    owner,
                    owner_token_account: owner_ata,
                    vault: vault_pda,
                    token_program: anchor_spl::token::ID,
                })
                .args(coinflip::instruction::Deposit { amount })
                .send()?;
            println!("Deposited {} tokens into vault", amount);
        }

        Commands::Flip { amount, side } => {
            let user = program.payer().pubkey();
            let user_ata = get_associated_token_address(&user, &{
                let info = program.rpc().get_account(&vault_pda)?;
                let account =
                    anchor_spl::token::TokenAccount::try_deserialize(&mut info.data.as_ref())?;
                account.mint
            });
            let tx = program
                .request()
                .accounts(coinflip::accounts::Flip {
                    state: state_pda,
                    vault_authority: vault_pda,
                    vault: vault_pda,
                    user,
                    user_token_account: user_ata,
                    token_program: anchor_spl::token::ID,
                })
                .args(coinflip::instruction::Flip {
                    amount,
                    side: side as u8,
                })
                .send()?;
            println!("Flipped coin. Transaction: {}", tx);
        }

        Commands::Withdraw { amount } => {
            let owner = program.payer().pubkey();
            let mint = {
                let info = program.rpc().get_account(&vault_pda)?;
                let account =
                    anchor_spl::token::TokenAccount::try_deserialize(&mut info.data.as_ref())?;
                account.mint
            };
            let owner_ata = get_associated_token_address(&owner, &mint);
            program
                .request()
                .accounts(coinflip::accounts::Withdraw {
                    state: state_pda,
                    vault_authority: vault_pda,
                    vault: vault_pda,
                    owner,
                    owner_token_account: owner_ata,
                    token_program: anchor_spl::token::ID,
                })
                .args(coinflip::instruction::Withdraw { amount })
                .send()?;
            println!("Withdrew {} tokens", amount);
        }
    }

    Ok(())
}
