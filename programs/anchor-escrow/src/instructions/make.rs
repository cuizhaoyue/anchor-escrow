use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer_checked, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{state::Escrow, errors::EscrowError};

/// `make` 指令的账户上下文。
/// 该指令会创建订单状态并把 token A 托管到 `vault`。
#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    /// 挂单发起人，负责支付本次新建账户的租金。
    #[account(mut)]
    pub maker: Signer<'info>,

    /// 订单状态账户（PDA）：
    /// - seeds = ["escrow", maker, seed]
    /// - 保存订单条款、交易双方资产 mint、期望收款数量等元数据
    #[account(
        init,
        payer = maker,
        space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE,
        seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    /// maker 要托管的资产 mint（token A）。
    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    /// maker 希望收到的资产 mint（token B）。
    #[account(mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    /// maker 持有 token A 的 ATA，`make` 时会从这里扣款到 `vault`。
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    /// 托管账户（vault）：
    /// - 是 mint_a 对应的 ATA
    /// - authority 为 `escrow` PDA，而不是 maker
    /// - 后续 take/refund 都通过 escrow PDA 签名来转出资产
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    // vault：由 escrow PDA 控制的 token A 账户，实际托管资产放在这里。
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    // 把订单信息写入 escrow 账户，后续 take/refund 都依赖这里的状态。
    fn populate_escrow(&mut self, seed: u64, receive: u64, bump: u8) -> Result<()> {
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump,
        });
        Ok(())
    }

    // maker -> vault 转入 amount 个 token A。
    // 使用 transfer_checked 会校验 mint 小数位，避免错误精度转账。
    fn deposit_tokens(&self, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.maker_ata_a.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.maker.to_account_info(),
                },
            ),
            amount,
            self.mint_a.decimals,
        )
    }
}

// make 指令入口：
// 1) 校验参数
// 2) 初始化并写入 escrow 状态
// 3) 把 token A 存入 vault
pub fn handler(ctx: Context<Make>, seed: u64, receive: u64, amount: u64) -> Result<()> {
    require!(receive > 0, EscrowError::InvalidAmount);
    require!(amount > 0, EscrowError::InvalidAmount);

    ctx.accounts
        .populate_escrow(seed, receive, ctx.bumps.escrow)?;
    ctx.accounts.deposit_tokens(amount)
}
