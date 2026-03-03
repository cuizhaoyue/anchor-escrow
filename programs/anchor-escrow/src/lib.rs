use anchor_lang::prelude::*;

mod state;
mod errors;

mod instructions;
use instructions::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod anchor_escrow {
    use super::*;

    // discriminator = 0：创建托管单，maker 存入 mint_a，等待 taker 用 mint_b 交换。
    #[instruction(discriminator = 0)]
    pub fn make(ctx: Context<Make>, seed: u64, receive: u64, amount: u64) -> Result<()> {
        instructions::make::handler(ctx, seed, receive, amount)
    }

    // discriminator = 1：taker 接单，支付 mint_b 给 maker，并取走 vault 内的 mint_a。
    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        instructions::take::handler(ctx)
    }

    // discriminator = 2：maker 撤单，取回 vault 中的 mint_a，并关闭 escrow。
    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        instructions::refund::handler(ctx)
    }
}
