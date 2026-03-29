import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { createToken2022Mint, createTokenAccount, mintTokens } from "./helpers/tokens";
import { airdropSol } from "./helpers/wallets";

describe("vogs-vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VogsVault as Program;
  const authority = Keypair.generate();
  const institution = Keypair.generate();
  const depositor = Keypair.generate();
  let mint: PublicKey;
  let vaultPda: PublicKey;

  before(async () => {
    await airdropSol(provider.connection, authority);
    await airdropSol(provider.connection, depositor);

    mint = await createToken2022Mint(provider.connection, authority, 6);

    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), institution.publicKey.toBuffer(), mint.toBuffer()],
      program.programId,
    );
  });

  it("initializes a vault", async () => {
    const vaultTokenAccount = await createTokenAccount(
      provider.connection,
      authority,
      mint,
      vaultPda,
    );

    await program.methods
      .initializeVault(1000) // 10% liquidity buffer
      .accounts({
        vault: vaultPda,
        mint,
        vaultTokenAccount: vaultTokenAccount.address,
        institution: institution.publicKey,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    const vault = await program.account.vault.fetch(vaultPda);
    expect(vault.liquidityBufferBps).to.equal(1000);
    expect(vault.totalDeposits.toNumber()).to.equal(0);
  });

  it("rejects allocations that don't sum to 10000", async () => {
    try {
      await program.methods
        .updateAllocations([
          { name: "Ondo USDY", targetBps: 4000, currentBps: 0, apyBps: 482 },
          { name: "Maple", targetBps: 3000, currentBps: 0, apyBps: 651 },
        ])
        .accounts({ vault: vaultPda, authority: authority.publicKey })
        .signers([authority])
        .rpc();
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidAllocationSum");
    }
  });

  it("accepts valid allocations summing to 10000", async () => {
    await program.methods
      .updateAllocations([
        { name: "Ondo USDY", targetBps: 4000, currentBps: 0, apyBps: 482 },
        { name: "Maple syrupUSDC", targetBps: 3500, currentBps: 0, apyBps: 651 },
        { name: "BlackRock BUIDL", targetBps: 2500, currentBps: 0, apyBps: 425 },
      ])
      .accounts({ vault: vaultPda, authority: authority.publicKey })
      .signers([authority])
      .rpc();

    const vault = await program.account.vault.fetch(vaultPda);
    expect(vault.allocations.length).to.equal(3);
  });
});
