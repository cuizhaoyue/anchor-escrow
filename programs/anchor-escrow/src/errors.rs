use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    // 输入数量（receive / amount）不能为 0。
    #[msg("Amount must be greater than zero.")]
    InvalidAmount,

    // escrow.maker 与传入账户不一致。
    #[msg("Escrow maker does not match the provided account.")]
    InvalidMaker,

    // escrow.mint_a 与传入账户不一致。
    #[msg("Escrow mint A does not match the provided account.")]
    InvalidMintA,
    
    // escrow.mint_b 与传入账户不一致。
    #[msg("Escrow mint B does not match the provided account.")]
    InvalidMintB,
}
