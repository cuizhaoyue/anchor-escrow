use anchor_lang::prelude::*;

mod state;
pub use state::*;

mod errors;
pub use errors::*;

mod instructions;
pub use instructions::*;

declare_id!("HJGfhRpfUJHwm6W7MvCyHU13qEXfgLkJnhSPD23Ushwu");

#[program]
pub mod anchor_escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}



