// #![cfg(feature = "test-bpf")]
use anyhow::anyhow;
use echo::state::{AuthorizedBufferHeader, VendingMachineBufferHeader};
// use solana_sdk::transaction::Transaction;
// use std::path::{Path, PathBuf};

use assert_matches::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_client::client_error::{ClientError/*, ClientErrorKind*/};
// use solana_client::rpc_client::RpcClient;
use solana_sdk::account::ReadableAccount;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::instruction::Instruction;
// use solana_sdk::message::Message;
// use solana_sdk::program_error::ProgramError;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
// use solana_sdk::rent::Rent;
use solana_sdk::signature::Keypair;
use solana_sdk::system_instruction;
use solana_sdk::system_program;
// use solana_sdk::sysvar;
use solana_sdk::{signature::Signer, transaction::Transaction};
use solana_validator::test_validator::*;
// use spl_token::instruction::initialize_mint;

use echo::instruction::EchoInstruction;

#[test]
fn test_echo() -> anyhow::Result<()> {
    solana_logger::setup_with_default("solana_program_runtime=debug");
    let program_id = Pubkey::new_unique();
    let echo_buffer = Keypair::new();

    // Set up the test validator
    let (test_validator, payer) = TestValidatorGenesis::default()
        .add_program("echo", program_id)
        .start();
    let rpc_client = test_validator.get_rpc_client();

    // let rpc_client = RpcClient::new_with_commitment("https://api.devnet.solana.com".to_string(), CommitmentLevel::confirmed());

    let blockhash = rpc_client.get_latest_blockhash().unwrap();

    // Create transaction
    let data: Vec<u8> = b"echo".to_vec();
    let mut transaction = Transaction::new_signed_with_payer(
        &[
            // Instruction to create buffer account
            system_instruction::create_account(
                &payer.pubkey(),
                &echo_buffer.pubkey(),
                rpc_client
                    .get_minimum_balance_for_rent_exemption(data.len())
                    .unwrap(),
                data.len() as u64, // allocate size of data to buffer account data
                &program_id,
            ),
            // Instruction to write to buffer
            Instruction {
                program_id,
                accounts: vec![AccountMeta::new(echo_buffer.pubkey(), false)],
                data: EchoInstruction::Echo { data }.try_to_vec()?,
            },
        ],
        Some(&payer.pubkey()),
        &vec![&payer, &echo_buffer],
        blockhash,
    );

    // Sign and send transaction
    transaction.sign(&[&payer, &echo_buffer], blockhash);
    rpc_client.send_and_confirm_transaction(&transaction)?;

    // Confirm that buffer data is correct
    let buffer = rpc_client.get_account(&echo_buffer.pubkey())?.data;
    println!("{:?}", buffer);
    let string = std::str::from_utf8(&buffer)?;
    println!("{:?}", string);
    assert_matches!(string, "echo");
    Ok(())
}

#[test]
fn test_echo_uninitialized() -> anyhow::Result<()> {
    solana_logger::setup_with_default("solana_program_runtime=debug");
    let program_id = Pubkey::new_unique();
    let echo_buffer = Keypair::new();

    // Set up the test validator
    let (test_validator, payer) = TestValidatorGenesis::default()
        .add_program("echo", program_id)
        .start();
    let rpc_client = test_validator.get_rpc_client();

    // let rpc_client = RpcClient::new_with_commitment("https://api.devnet.solana.com".to_string(), CommitmentLevel::confirmed());

    let blockhash = rpc_client.get_latest_blockhash().unwrap();

    // Create transaction
    let data: Vec<u8> = b"echo".to_vec();
    let mut transaction = Transaction::new_signed_with_payer(
        &[
            // Instruction to write to buffer
            Instruction {
                program_id,
                accounts: vec![AccountMeta::new(echo_buffer.pubkey(), false)],
                data: EchoInstruction::Echo { data }.try_to_vec()?,
            },
        ],
        Some(&payer.pubkey()),
        &vec![&payer],
        blockhash,
    );

    // Sign and send transaction
    transaction.sign(&[&payer], blockhash);
    let e = rpc_client
        .send_and_confirm_transaction(&transaction)
        .unwrap_err();
    println!("{:?}", e);
    assert_matches!(e, ClientError { .. });

    Ok(())
}

#[test]
fn test_echo_nonzero() -> anyhow::Result<()> {
    solana_logger::setup_with_default("solana_program_runtime=debug");
    let program_id = Pubkey::new_unique();
    let echo_buffer = Keypair::new();

    let (test_validator, payer) = TestValidatorGenesis::default()
        .add_program("echo", program_id)
        .start();
    let rpc_client = test_validator.get_rpc_client();

    // let rpc_client = RpcClient::new_with_commitment("https://api.devnet.solana.com".to_string(), CommitmentLevel::confirmed());

    let blockhash = rpc_client.get_latest_blockhash().unwrap();

    let data: Vec<u8> = b"echo".to_vec();
    let data2: Vec<u8> = data.clone();
    let mut transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &echo_buffer.pubkey(),
                rpc_client
                    .get_minimum_balance_for_rent_exemption(data.len())
                    .unwrap(),
                4,
                &program_id,
            ),
            Instruction {
                program_id,
                accounts: vec![AccountMeta::new(echo_buffer.pubkey(), false)],
                data: EchoInstruction::Echo { data }.try_to_vec().unwrap(),
            },
            Instruction {
                program_id,
                accounts: vec![AccountMeta::new(echo_buffer.pubkey(), false)],
                data: EchoInstruction::Echo { data: data2 }.try_to_vec().unwrap(),
            },
        ],
        Some(&payer.pubkey()),
        &vec![&payer, &echo_buffer],
        blockhash,
    );
    let blockhash = rpc_client.get_latest_blockhash().unwrap();

    transaction.sign(&[&payer, &echo_buffer], blockhash);
    let result = rpc_client.send_and_confirm_transaction(&transaction);
    match result {
        Ok(_) => Err(anyhow!("Should have failed")),
        Err(_) => Ok(()),
    }
}

#[test]
fn test_authorized_echo() -> anyhow::Result<()> {
    solana_logger::setup_with_default("solana_program_runtime=debug");
    let program_id = Pubkey::new_unique();

    let (test_validator, payer) = TestValidatorGenesis::default()
        .add_program("echo", program_id)
        .start();
    let rpc_client = test_validator.get_rpc_client();

    let buffer_seed = 1u64;
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"authority",
            payer.pubkey().as_ref(),
            &buffer_seed.to_le_bytes(),
        ],
        &program_id,
    );

    let data = b"authorized".to_vec();

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(pda, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: EchoInstruction::InitializeAuthorizedEcho {
                buffer_seed,
                buffer_size: 13 + data.len(),
            }
            .try_to_vec()?,
        }],
        Some(&payer.pubkey()),
        &vec![&payer],
        blockhash,
    );
    transaction.sign(&[&payer], blockhash);
    rpc_client.send_and_confirm_transaction(&transaction)?;
    // let account = rpc_client.get_account(&pda)?;

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(pda, false),
                AccountMeta::new_readonly(payer.pubkey(), true),
            ],
            data: EchoInstruction::AuthorizedEcho { data }.try_to_vec()?,
        }],
        Some(&payer.pubkey()),
        &vec![&payer],
        blockhash,
    );
    transaction.sign(&[&payer], blockhash);
    rpc_client.send_and_confirm_transaction(&transaction)?;
    let echo_data = rpc_client.get_account(&pda)?.data;
    let echo_buffer = AuthorizedBufferHeader::try_from_slice(&echo_data)?.echo_data;
    let string = std::str::from_utf8(&echo_buffer)?;
    assert_matches!(string, "authorized");
    Ok(())
}

#[test]
fn test_vending_machine() -> anyhow::Result<()> {
    solana_logger::setup_with("solana_runtime::message_processor=debug");
    let program_id = Pubkey::new_unique();
    let vending_machine_mint = Keypair::new();
    let user_token_account = Keypair::new();

    let (test_validator, payer) = TestValidatorGenesis::default()
        .add_program("echo", program_id)
        .start();
    let rpc_client = test_validator.get_rpc_client();

    let price = 42u64;
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"vending_machine",
            vending_machine_mint.pubkey().as_ref(),
            &price.to_le_bytes(),
        ],
        &program_id,
    );

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &vending_machine_mint.pubkey(),
                rpc_client.get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &vending_machine_mint.pubkey(),
                &payer.pubkey(),
                None,
                spl_token::native_mint::DECIMALS,
            )?,
            system_instruction::create_account(
                &payer.pubkey(),
                &user_token_account.pubkey(),
                rpc_client
                    .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &user_token_account.pubkey(),
                &vending_machine_mint.pubkey(),
                &payer.pubkey(),
            )?,
            spl_token::instruction::mint_to(
                &spl_token::id(),
                &vending_machine_mint.pubkey(),
                &user_token_account.pubkey(),
                &payer.pubkey(),
                &[&payer.pubkey()],
                42,
            )?,
            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(pda, false),
                    AccountMeta::new_readonly(vending_machine_mint.pubkey(), false),
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data: EchoInstruction::InitializeVendingMachineEcho {
                    price,
                    buffer_size: b"vending_machine".len() + 4 + 9,
                }
                .try_to_vec()?,
            },
        ],
        Some(&payer.pubkey()),
        &vec![&payer, &vending_machine_mint, &user_token_account],
        blockhash,
    );
    transaction.sign(
        &[&payer, &vending_machine_mint, &user_token_account],
        blockhash,
    );
    rpc_client.send_and_confirm_transaction(&transaction)?;
    let ta_initial_amount = spl_token::state::Account::unpack(
        rpc_client.get_account(&user_token_account.pubkey())?.data(),
    )?
    .amount;
    // let user_token_account_info = rpc_client.get_account(&user_token_account.pubkey())?.data();
    let vending_machine_buffer = rpc_client.get_account(&pda)?;
    println!("{:?}", vending_machine_buffer.data);

    let blockhash = rpc_client.get_latest_blockhash()?;
    let mut transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(pda, false),
                AccountMeta::new_readonly(payer.pubkey(), true),
                AccountMeta::new(user_token_account.pubkey(), false),
                AccountMeta::new(vending_machine_mint.pubkey(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: EchoInstruction::VendingMachineEcho {
                data: b"vending machine".to_vec(),
            }
            .try_to_vec()?,
        }],
        Some(&payer.pubkey()),
        &vec![&payer],
        blockhash,
    );
    transaction.sign(&[&payer], blockhash);
    rpc_client.send_and_confirm_transaction(&transaction)?;
    let ta_final_amount = spl_token::state::Account::unpack(
        rpc_client.get_account(&user_token_account.pubkey())?.data(),
    )?
    .amount;
    assert!(ta_final_amount == ta_initial_amount - price);
    let vm_data = rpc_client.get_account(&pda)?.data;
    let vm_buffer = VendingMachineBufferHeader::try_from_slice(&vm_data)?.echo_data;
    let string = std::str::from_utf8(&vm_buffer)?;
    assert_matches!(string, "vending machine");

    Ok(())
}