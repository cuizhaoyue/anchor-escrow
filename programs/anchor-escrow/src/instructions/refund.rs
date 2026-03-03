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
pub struct Refund<'info> {
    // 挂单人本人撤单，必须签名。
    #[account(mut)]
    pub maker: Signer<'info>,

    // Escrow PDA：校验 maker 和 mint_a 一致后，执行关闭。
    #[account(
        mut,
        close = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = maker @ EscrowError::InvalidMaker,
        has_one = mint_a @ EscrowError::InvalidMintA,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    // token A mint（被托管资产）。
    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    // escrow 控制的 token A 托管账户。
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // maker 收回 token A 的 ATA（不存在则自动创建）。
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    
    pub maker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    // 把 vault 全部 token A 退回 maker，并关闭 vault。
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
                    to: self.maker_ata_a.to_account_info(),
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
                // vault 关闭后的租金返还给 maker。
                destination: self.maker.to_account_info(),
                authority: self.escrow.to_account_info(),
            },
            signer_seeds,
        ))
    }
}

// refund 指令入口：
// 1) maker 取回全部 token A
// 2) 关闭 vault
// 3) 指令结束时由 close = maker 自动关闭 escrow
pub fn handler(ctx: Context<Refund>) -> Result<()> {
    ctx.accounts.withdraw_and_close_vault()
}
