use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, FromPrimitive, PartialEq)]
pub enum EchoError {
    #[error("Instruction not implemented.")]
    NotImplemented,

    #[error("Echo_Buffer has non-zero data")]
    NonZeroData,

    #[error("Authority must be signer")]
    AuthorityNotSigner,

    #[error("Invalid Authority")]
    InvalidAuthority,

    #[error("Invalid Authorized_buffer_key")]
    InvalidAuthorizedBuffer,
}

impl From<EchoError> for ProgramError {
    fn from(e: EchoError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
