use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{pubkey::Pubkey};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct AuthorizedBufferHeader {
    pub bump_seed: u8,
    pub buffer_seed: u64,
    pub echo_data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct VendingMachineBufferHeader {
    pub bump_seed: u8,
    pub price: u64,
    pub vending_machine_mint: Pubkey,
    pub echo_data: Vec<u8>,
}
