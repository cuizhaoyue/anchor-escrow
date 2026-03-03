use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    // 用户自定义种子，用于支持同一个 maker 创建多笔托管单。
    pub seed: u64,

    // 挂单人地址（托管单所有者）。
    pub maker: Pubkey,

    // maker 存入的代币 Mint（卖出的资产）。
    pub mint_a: Pubkey,

    // maker 希望收到的代币 Mint（买入的资产）。
    pub mint_b: Pubkey,

    // taker 需要支付给 maker 的 mint_b 数量。
    pub receive: u64,

    // Escrow PDA bump，用于程序签名时复原 PDA seeds。
    pub bump: u8,
}
