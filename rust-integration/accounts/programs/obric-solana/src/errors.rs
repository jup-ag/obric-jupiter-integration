use anchor_lang::error_code;

#[error_code]
pub enum ObricError {
    AlreadyInitialized,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Mismatched token mint")]
    MismatchedTokenMint,
    #[msg("Invalid concentration argument")]
    InvalidConcentrationArg,
    InsufficientActiveY,
    InsufficientActiveX,
    #[msg("input amount must be greater than 0")]
    InvalidInputAmount,
    NumOverflowing,
    OutputAmountLessThanExpected,

    #[msg("Pyth has an internal error")]
    PythError,
    #[msg("Pyth price oracle is offline")]
    PythOffline,
    #[msg("Program should not try to serialize a price account")]
    TryToSerializePriceAccount,
    InvalidPriceAccount,
    NegativePrice,
    #[msg("Invalid price account owner program")]
    InvalidPriceOwner,

    InvalidRoutesForSwap,
    PoolLocked,
}
