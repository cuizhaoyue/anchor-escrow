import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { AnchorEscrow } from "../target/types/anchor_escrow";
import { expect } from "chai";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";

type CaseContext = {
  admin: Keypair;
  maker: Keypair;
  taker: Keypair;
  mintA: PublicKey;
  mintB: PublicKey;
  makerAtaA: PublicKey;
  takerAtaB: PublicKey;
};

const DECIMALS = 6;
const INITIAL_MAKER_A = 5_000_000;
const INITIAL_TAKER_B = 5_000_000;

describe("anchor-escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.anchorEscrow as Program<AnchorEscrow>;
  const connection = provider.connection;

  let ctx: CaseContext;

  async function airdrop(pubkey: PublicKey, sol: number): Promise<void> {
    const signature = await connection.requestAirdrop(
      pubkey,
      sol * LAMPORTS_PER_SOL
    );
    const blockhash = await connection.getLatestBlockhash();
    await connection.confirmTransaction(
      {
        signature,
        blockhash: blockhash.blockhash,
        lastValidBlockHeight: blockhash.lastValidBlockHeight,
      },
      "confirmed"
    );
  }

  function deriveEscrowPda(maker: PublicKey, seed: number): PublicKey {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        maker.toBuffer(),
        new BN(seed).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];
  }

  function deriveAta(
    mint: PublicKey,
    owner: PublicKey,
    ownerOffCurve = false
  ): PublicKey {
    return getAssociatedTokenAddressSync(
      mint,
      owner,
      ownerOffCurve,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
  }

  async function makeEscrow(
    seed: number,
    receive: number,
    amount: number
  ): Promise<{
    escrow: PublicKey;
    vault: PublicKey;
  }> {
    const escrow = deriveEscrowPda(ctx.maker.publicKey, seed);
    const vault = deriveAta(ctx.mintA, escrow, true);

    await createAssociatedTokenAccount(
      connection,
      ctx.admin,
      ctx.mintA,
      escrow,
      undefined,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      true
    );

    const signature = await program.methods
      .make(new BN(seed), new BN(receive), new BN(amount))
      .accounts({
        maker: ctx.maker.publicKey,
        escrow,
        mintA: ctx.mintA,
        mintB: ctx.mintB,
        makerAtaA: ctx.makerAtaA,
        vault,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([ctx.maker])
      .rpc();
    await connection.confirmTransaction(signature, "confirmed");

    return { escrow, vault };
  }

  function extractErrorMessage(error: unknown): string {
    if (typeof error === "string") {
      return error;
    }
    if (error && typeof error === "object") {
      const anchorErr = (error as { error?: { errorMessage?: string } }).error;
      if (anchorErr?.errorMessage) {
        return anchorErr.errorMessage;
      }
      const message = (error as { message?: string }).message;
      if (message) {
        return message;
      }
    }
    return String(error);
  }

  beforeEach(async () => {
    ctx = {
      admin: Keypair.generate(),
      maker: Keypair.generate(),
      taker: Keypair.generate(),
      mintA: PublicKey.default,
      mintB: PublicKey.default,
      makerAtaA: PublicKey.default,
      takerAtaB: PublicKey.default,
    };

    await airdrop(ctx.admin.publicKey, 4);
    await airdrop(ctx.maker.publicKey, 2);
    await airdrop(ctx.taker.publicKey, 2);

    ctx.mintA = await createMint(
      connection,
      ctx.admin,
      ctx.admin.publicKey,
      null,
      DECIMALS,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID
    );
    ctx.mintB = await createMint(
      connection,
      ctx.admin,
      ctx.admin.publicKey,
      null,
      DECIMALS,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID
    );

    ctx.makerAtaA = await createAssociatedTokenAccount(
      connection,
      ctx.admin,
      ctx.mintA,
      ctx.maker.publicKey,
      undefined,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    ctx.takerAtaB = await createAssociatedTokenAccount(
      connection,
      ctx.admin,
      ctx.mintB,
      ctx.taker.publicKey,
      undefined,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    await mintTo(
      connection,
      ctx.admin,
      ctx.mintA,
      ctx.makerAtaA,
      ctx.admin,
      INITIAL_MAKER_A,
      [],
      undefined,
      TOKEN_PROGRAM_ID
    );
    await mintTo(
      connection,
      ctx.admin,
      ctx.mintB,
      ctx.takerAtaB,
      ctx.admin,
      INITIAL_TAKER_B,
      [],
      undefined,
      TOKEN_PROGRAM_ID
    );
  });

  it("make: should create escrow and move token A into vault", async () => {
    const seed = 101;
    const receive = 800_000;
    const amount = 1_500_000;
    const makerBefore = await getAccount(
      connection,
      ctx.makerAtaA,
      "confirmed",
      TOKEN_PROGRAM_ID
    );

    const { escrow, vault } = await makeEscrow(seed, receive, amount);

    const escrowState = await program.account.escrow.fetch(escrow);
    expect(escrowState.seed.toString()).to.eq(String(seed));
    expect(escrowState.maker.toBase58()).to.eq(ctx.maker.publicKey.toBase58());
    expect(escrowState.mintA.toBase58()).to.eq(ctx.mintA.toBase58());
    expect(escrowState.mintB.toBase58()).to.eq(ctx.mintB.toBase58());
    expect(escrowState.receive.toString()).to.eq(String(receive));

    const makerAfter = await getAccount(
      connection,
      ctx.makerAtaA,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
    const vaultAfter = await getAccount(
      connection,
      vault,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
    expect(Number(makerBefore.amount - makerAfter.amount)).to.eq(amount);
    expect(Number(vaultAfter.amount)).to.eq(amount);
  });

  it("take: should exchange tokens and close vault/escrow", async () => {
    const seed = 202;
    const receive = 650_000;
    const amount = 1_200_000;

    const { escrow, vault } = await makeEscrow(seed, receive, amount);

    const takerAtaA = deriveAta(ctx.mintA, ctx.taker.publicKey);
    const makerAtaB = deriveAta(ctx.mintB, ctx.maker.publicKey);

    const takerBeforeB = await getAccount(
      connection,
      ctx.takerAtaB,
      "confirmed",
      TOKEN_PROGRAM_ID
    );

    const signature = await program.methods
      .take()
      .accounts({
        taker: ctx.taker.publicKey,
        maker: ctx.maker.publicKey,
        escrow,
        mintA: ctx.mintA,
        mintB: ctx.mintB,
        vault,
        takerAtaA,
        takerAtaB: ctx.takerAtaB,
        makerAtaB,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([ctx.taker])
      .rpc();
    await connection.confirmTransaction(signature, "confirmed");

    const takerAfterA = await getAccount(
      connection,
      takerAtaA,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
    const takerAfterB = await getAccount(
      connection,
      ctx.takerAtaB,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
    const makerAfterB = await getAccount(
      connection,
      makerAtaB,
      "confirmed",
      TOKEN_PROGRAM_ID
    );

    expect(Number(takerAfterA.amount)).to.eq(amount);
    expect(Number(takerBeforeB.amount - takerAfterB.amount)).to.eq(receive);
    expect(Number(makerAfterB.amount)).to.eq(receive);
    expect(await connection.getAccountInfo(vault, "confirmed")).to.eq(null);
    expect(await connection.getAccountInfo(escrow, "confirmed")).to.eq(null);
  });

  it("refund: should return token A to maker and close vault/escrow", async () => {
    const seed = 303;
    const receive = 700_000;
    const amount = 900_000;
    const makerInitial = await getAccount(
      connection,
      ctx.makerAtaA,
      "confirmed",
      TOKEN_PROGRAM_ID
    );

    const { escrow, vault } = await makeEscrow(seed, receive, amount);

    const signature = await program.methods
      .refund()
      .accounts({
        maker: ctx.maker.publicKey,
        escrow,
        mintA: ctx.mintA,
        vault,
        makerAtaA: ctx.makerAtaA,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([ctx.maker])
      .rpc();
    await connection.confirmTransaction(signature, "confirmed");

    const makerAfter = await getAccount(
      connection,
      ctx.makerAtaA,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
    expect(Number(makerAfter.amount)).to.eq(Number(makerInitial.amount));
    expect(await connection.getAccountInfo(vault, "confirmed")).to.eq(null);
    expect(await connection.getAccountInfo(escrow, "confirmed")).to.eq(null);
  });

  it("make: should fail when receive is zero", async () => {
    const seed = 404;
    const receive = 0;
    const amount = 100_000;
    const escrow = deriveEscrowPda(ctx.maker.publicKey, seed);
    const vault = deriveAta(ctx.mintA, escrow, true);
    await createAssociatedTokenAccount(
      connection,
      ctx.admin,
      ctx.mintA,
      escrow,
      undefined,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      true
    );

    let caught: unknown = null;
    try {
      await program.methods
        .make(new BN(seed), new BN(receive), new BN(amount))
        .accounts({
          maker: ctx.maker.publicKey,
          escrow,
          mintA: ctx.mintA,
          mintB: ctx.mintB,
          makerAtaA: ctx.makerAtaA,
          vault,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([ctx.maker])
        .rpc();
    } catch (error) {
      caught = error;
    }

    expect(caught).to.not.eq(null);
    const message = extractErrorMessage(caught);
    expect(message).to.satisfy(
      (m: string) =>
        m.includes("Amount must be greater than zero.") ||
        m.includes("custom program error: 0x1770")
    );
  });
});
