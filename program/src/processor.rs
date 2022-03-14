use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo}, 
    entrypoint::ProgramResult, msg, 
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    program::{invoke, invoke_signed},
    sysvar::{rent::Rent, Sysvar},
};
// use solana_sdk::account::WritableAccount;

use spl_token::instruction::burn;

use crate::error::EchoError;
use crate::instruction::EchoInstruction;
use crate::state::{AuthorizedBufferHeader, VendingMachineBufferHeader};

pub struct Processor {}

pub fn assert_with_msg(statement: bool, err: ProgramError, msg: &str) -> ProgramResult {
    if !statement {
        msg!(msg);
        Err(err)
    } else {
        Ok(())
    }
}


pub fn assert_is_writable(account_info: &AccountInfo) -> ProgramResult {
    assert_with_msg(
        account_info.is_writable,
        ProgramError::InvalidArgument,
        &format!("Account {} must be writable.", account_info.key),
    )
}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = EchoInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction {
            EchoInstruction::Echo { data } => {
                msg!("Instruction: Echo");
                let accounts_iter = &mut accounts.iter();
                let echo_buffer = next_account_info(accounts_iter)?;

                if echo_buffer.data_len() == 0 {
                    return Err(EchoError::NonZeroData.into());
                }

                let mut echo_data = echo_buffer.try_borrow_mut_data()?; 
                for &mut dat in echo_data.into_iter() {
                    if dat != 0u8 {
                        return Err(EchoError::NonZeroData.into());
                    }
                }
                    
                if echo_data.len() > data.len() {
                    echo_data.copy_from_slice(&data);
                } else {
                    let echo_len = echo_data.len();
                    echo_data.copy_from_slice(&data[..echo_len]);
                }
                Ok(())
            }


            EchoInstruction::InitializeAuthorizedEcho {
                buffer_seed,
                buffer_size,
            } => {
                msg!("Instruction: InitializeAuthorizedEcho");
                
                // accounts
                let accounts_iter = &mut accounts.iter();
                let authorized_buffer = next_account_info(accounts_iter)?;
                let authority = next_account_info(accounts_iter)?;
                // let system_program = next_account_info(accounts_iter)?;

                
                // check signer 
                if !authority.is_signer {
                    return Err(EchoError::AuthorityNotSigner.into())
                }

                // allocate buffer_size bytes to the authorized_buffer account and assign it the Echo Program.
                let (authorized_buffer_key, bump_seed) = Pubkey::find_program_address(
                    &[
                        b"authority",
                        authority.key.as_ref(),
                        &buffer_seed.to_le_bytes()
                ],
                    program_id,
                );

                // check authorized_buffer_key is same as authorized_buffer
                if authorized_buffer_key != *authorized_buffer.key {
                    return Err(EchoError::InvalidAuthorizedBuffer.into())
                }

                // CPI to the system program
                invoke_signed(
                    &system_instruction::create_account(
                        authority.key,
                        authorized_buffer.key,
                        Rent::get()?.minimum_balance(buffer_size) as u64,
                        buffer_size as u64,
                        program_id,
                    ),
                    &[authority.clone(), authorized_buffer.clone()],
                    &[&[b"authority", authority.key.as_ref(), &buffer_seed.to_le_bytes(), &[bump_seed]]],
                )?;

                // Setting up authorized buffer
                // byte 0: bump_seed
                // bytes 1-8: buffer_seed
                let echo_data = vec![0; buffer_size - 9 - 4];
                let buffer_data = AuthorizedBufferHeader { bump_seed, buffer_seed, echo_data };
                let mut authorized_buffer_data = authorized_buffer.try_borrow_mut_data()?;
                buffer_data.serialize(&mut *authorized_buffer_data)?;
                
                Ok(())
            }


            EchoInstruction::AuthorizedEcho { data} => {
                msg!("Instruction: AuthorizedEcho");
                let accounts_iter = &mut accounts.iter();
                let authorized_buffer = next_account_info(accounts_iter)?;
                let authority = next_account_info(accounts_iter)?;

                // check signer 
                if !authority.is_signer {
                    return Err(EchoError::AuthorityNotSigner.into())
                }

                let mut buffer_data = AuthorizedBufferHeader::try_from_slice(&authorized_buffer.data.borrow())?; 

                let authority_seeds = &[b"authority", authority.key.as_ref(), &buffer_data.buffer_seed.to_le_bytes(), &[buffer_data.bump_seed]];
                let authorized_buffer_key = Pubkey::create_program_address(authority_seeds, program_id)?;

                // Invalid Authority Error
                if authorized_buffer_key != *authorized_buffer.key {
                    return Err(EchoError::InvalidAuthority.into())
                }
                
                // Zero out all the data
                buffer_data.echo_data.fill(0);

                // Copy data in to authorized_buffer
                let min_of_len = std::cmp::min(buffer_data.echo_data.len(), data.len());
                buffer_data.echo_data.copy_from_slice(&data[..min_of_len]);
                buffer_data.serialize(&mut *authorized_buffer.data.borrow_mut())?;
                msg!("end");

                Ok(())
            }
            
            EchoInstruction::InitializeVendingMachineEcho {
                price,
                buffer_size,
            } => {
                msg!("Instruction: InitializeVendingMachineEcho");
                let accounts_iter = &mut accounts.iter();
                let vending_machine_buffer = next_account_info(accounts_iter)?;
                let vending_machine_mint = next_account_info(accounts_iter)?;
                let payer = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;

                // check signer 
                if !payer.is_signer {
                    return Err(EchoError::AuthorityNotSigner.into())
                }

                assert_is_writable(vending_machine_buffer)?;

                let (authorithed_buffer_key, bump_seed) = Pubkey::find_program_address(
                    &[
                        b"vending_machine",
                        vending_machine_mint.key.as_ref(),
                        &price.to_le_bytes(),
                    ],
                    program_id,
                );

                // Check Authority
                if authorithed_buffer_key != *vending_machine_buffer.key {
                    return Err(EchoError::InvalidAuthorizedBuffer.into())
                }

                // CPI to the system program
                invoke_signed(
                    &system_instruction::create_account(
                        payer.key,
                        vending_machine_buffer.key,
                        Rent::get()?.minimum_balance(buffer_size) as u64,
                        buffer_size as u64,
                        program_id,
                    ),
                    &[payer.clone(), vending_machine_buffer.clone()],
                    &[&[b"authority", vending_machine_mint.key.as_ref(), &price.to_le_bytes(), &[bump_seed]]],
                )?;

                // Setting up authorized buffer
                let echo_data = vec![0; buffer_size - 1 - 8 - 4 - 4];
                let buffer_data = VendingMachineBufferHeader { bump_seed, price, vending_machine_mint:*vending_machine_mint.key, echo_data };
                let mut vending_buffer_data = vending_machine_buffer.try_borrow_mut_data()?;
                buffer_data.serialize(&mut *vending_buffer_data)?;

                msg!("Instruction: InitializeVendingMachineEcho END & SUCCESS");
                Ok(())
            }


            EchoInstruction::VendingMachineEcho { data} => {
                msg!("Instruction: VendingMachineEcho");
                let accounts_iter = &mut accounts.iter();
                let vending_machine_buffer = next_account_info(accounts_iter)?;
                let user = next_account_info(accounts_iter)?;
                let user_token_account = next_account_info(accounts_iter)?;
                let vending_machine_mint = next_account_info(accounts_iter)?;
                let token_program = next_account_info(accounts_iter)?;
                

                if !user.is_signer {
                    return Err(EchoError::AuthorityNotSigner.into());
                }

                assert_is_writable(vending_machine_buffer)?;
                assert_is_writable(user_token_account)?;
                assert_is_writable(vending_machine_mint)?;

                let mut vending_buffer = VendingMachineBufferHeader::try_from_slice(&vending_machine_buffer.data.borrow())?;

                let vending_seeds = &[b"vending_machine",vending_machine_mint.key.as_ref(), &vending_buffer.price.to_le_bytes(), &[vending_buffer.bump_seed]];                
                let vending_buffer_key = Pubkey::create_program_address(vending_seeds, program_id)?;

                if vending_buffer_key != *vending_machine_buffer.key {
                    return Err(EchoError::InvalidAuthority.into());
                }

                // Burn price amount of tokens from user_token_account
                invoke(
                    &burn(
                        &spl_token::id(),
                        user_token_account.key,
                        vending_machine_mint.key,
                        user.key,
                        &[user.key],
                        vending_buffer.price
                    )?,
                    &[user_token_account.clone(), vending_machine_mint.clone(), user.clone()],
                )?;



                vending_buffer.echo_data.fill(0);
                let min_of_len = std::cmp::min(vending_buffer.echo_data.len(), data.len());
                vending_buffer.echo_data.copy_from_slice(&data[..min_of_len]);
                vending_buffer.serialize(&mut *vending_machine_buffer.data.borrow_mut())?;


                
                Ok(())
            }
        }
        // Ok(())
    }
}
