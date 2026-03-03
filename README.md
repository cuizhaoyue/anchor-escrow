# Anchor Escrow

> Solana 托管交易程序 - Blueshift 挑战项目

## 挑战链接

https://learn.blueshift.gg/zh-CN/challenges/anchor-escrow

## 项目概述

托管服务（Escrow）是一种强大的金融工具，可以在两方之间实现安全的代币交换。它就像一个数字保险箱，一方用户可以锁定代币 A，等待另一方用户存入代币 B，然后完成交换。这创造了一个无需信任的环境，双方都不需要担心对方会退出交易。

## 功能特性

程序通过三个核心指令实现托管交易：

### 1. Make (创建托管单)
- 创建者（Maker）定义交易条款
- 将约定数量的代币 A 存入保险库（Vault）
- 设置希望收到的代币 B 及数量
- 使用种子随机数支持同一用户创建多笔托管

### 2. Take (接受托管)
- 接受者（Taker）支付约定数量的代币 B 给创建者
- 作为回报，获得保险库中锁定的代币 A
- 托管账户自动关闭

### 3. Refund (退款)
- 创建者可取消托管报价
- 取回保险库中的代币 A
- 托管账户自动关闭

## Escrow 状态结构

```rust
pub struct Escrow {
    pub seed: u64,      // 用户自定义种子
    pub maker: Pubkey,  // 挂单人地址
    pub mint_a: Pubkey, // 存入的代币
    pub mint_b: Pubkey, // 期望收到的代币
    pub receive: u64,   // taker 需支付的数量
    pub bump: u8,       // PDA bump
}
```

## 项目结构

```
anchor-escrow/
├── programs/
│   └── anchor-escrow/
│       └── src/
│           ├── instructions/
│           │   ├── make.rs      # 创建托管指令
│           │   ├── take.rs      # 接受托管指令
│           │   ├── refund.rs    # 退款指令
│           │   └── mod.rs       # 模块导出
│           ├── state.rs         # Escrow 状态定义
│           ├── errors.rs        # 自定义错误
│           └── lib.rs           # 程序入口
├── tests/                       # 测试文件
├── Anchor.toml                  # Anchor 配置
└── Cargo.toml                   # Rust 依赖配置
```

## 环境要求

- Rust
- Solana CLI
- Anchor 0.31.0+

## 安装

```bash
# 安装依赖
anchor build

# 配置本地验证器
solana-test-validator
```

## 构建

```bash
# 构建程序
anchor build

# 部署到本地网络
anchor deploy
```

## 测试

```bash
# 运行测试
anchor test
```

## 自定义指令 Discriminator

本程序使用自定义指令 discriminator：

- `discriminator = 0`: Make 指令
- `discriminator = 1`: Take 指令
- `discriminator = 2`: Refund 指令

## 错误处理

| 错误代码 | 说明 |
|---------|------|
| `InvalidAmount` | 输入数量必须大于零 |
| `InvalidMaker` | 托管创建者与传入账户不匹配 |
| `InvalidMintA` | 托管的 mint_a 与传入账户不匹配 |
| `InvalidMintB` | 托管的 mint_b 与传入账户不匹配 |

## 程序 ID

- Localnet: `HJGfhRpfUJHwm6W7MvCyHU13qEXfgLkJnhSPD23Ushwu`

## 许可证

MIT
