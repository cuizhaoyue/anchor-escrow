use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::{state::Escrow, errors::EscrowError};

#[derive(Accounts)]
pub struct Take<'info> {
    // 接单人：支付可能的 ATA 初始化费用，并签名授权支付 token B。
    #[account(mut)]
    pub taker: Signer<'info>,

    // 挂单人：接收 token B，也作为 escrow/vault 关闭后的租金回收地址。
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    // Escrow PDA：订单主体。
    // has_one 约束确保传入的 maker/mint_a/mint_b 与挂单时完全一致，防止错配盗提。
    #[account(
        mut,
        close = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = maker @ EscrowError::InvalidMaker,
        has_one = mint_a @ EscrowError::InvalidMintA,
        has_one = mint_b @ EscrowError::InvalidMintB,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    // token A mint（卖方资产）。
    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    // token B mint（买方支付资产）。
    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    // escrow 控制的 token A 托管账户。
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // taker 接收 token A 的 ATA（不存在则自动创建）。
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    // taker 支付 token B 的 ATA（必须可写，余额会减少）。
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // maker 接收 token B 的 ATA（不存在则自动创建）。
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    // 第一步：taker 按 escrow.receive 支付 token B 给 maker。
    fn transfer_to_maker(&mut self) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_ata_b.to_account_info(),
                    mint: self.mint_b.to_account_info(),
                    to: self.maker_ata_b.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            self.escrow.receive,
            self.mint_b.decimals,
        )
    }

    // 第二步：从 vault 提走全部 token A 给 taker，并关闭 vault。
    // 因为 vault authority 是 escrow PDA，所以 CPI 需要 PDA signer seeds。
    fn withdraw_and_close_vault(&mut self) -> Result<()> {
        let seed_bytes = self.escrow.seed.to_le_bytes();
        let maker_key = self.maker.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            maker_key.as_ref(),
            seed_bytes.as_ref(),
            &[self.escrow.bump],
        ]];

        let amount = self.vault.amount;
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.vault.to_account_info(),
                    mint: self.mint_a.to_account_info(),
                    to: self.taker_ata_a.to_account_info(),
                    authority: self.escrow.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            self.mint_a.decimals,
        )?;

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                // 关闭 token 账户后返还的租金给 maker（与教程流程一致）。
                destination: self.maker.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        ))
    }
}

// take 指令入口：
// 1) taker 先支付 token B
// 2) taker 再领取托管 token A 并关闭 vault
// 3) 指令结束时由 close = maker 自动关闭 escrow
pub fn handler(ctx: Context<Take>) -> Result<()> {
    ctx.accounts.transfer_to_maker()?;
    ctx.accounts.withdraw_and_close_vault()
}
