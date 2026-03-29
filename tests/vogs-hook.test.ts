import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { createToken2022Mint, createTokenAccount, mintTokens } from "./helpers/tokens";
import { airdropSol } from "./helpers/wallets";

describe("vogs-hook", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VogsHook as Program;
  const authority = Keypair.generate();
  const sender = Keypair.generate();
  const receiver = Keypair.generate();
  const flaggedWallet = Keypair.generate();
  let mint: PublicKey;

  before(async () => {
    await airdropSol(provider.connection, authority);
    await airdropSol(provider.connection, sender);
    await airdropSol(provider.connection, receiver);
  });

  it("initializes the hook config", async () => {
    const gatekeeperNetwork = Keypair.generate().publicKey;
    mint = await createToken2022Mint(
      provider.connection,
      authority,
      6,
      program.programId,
    );

    const [hookConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("hook-config"), mint.toBuffer()],
      program.programId,
    );

    await program.methods
      .initialize(gatekeeperNetwork)
      .accounts({
        hookConfig,
        mint,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const config = await program.account.hookConfig.fetch(hookConfig);
    expect(config.authority.toString()).to.equal(authority.publicKey.toString());
    expect(config.mint.toString()).to.equal(mint.toString());
  });

  it("adds a wallet to the blocklist", async () => {
    const [blocklistEntry] = PublicKey.findProgramAddressSync(
      [Buffer.from("blocklist"), mint.toBuffer(), flaggedWallet.publicKey.toBuffer()],
      program.programId,
    );
    const [hookConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("hook-config"), mint.toBuffer()],
      program.programId,
    );

    await program.methods
      .addToBlocklist()
      .accounts({
        blocklistEntry,
        hookConfig,
        mint,
        wallet: flaggedWallet.publicKey,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const entry = await program.account.blocklistEntry.fetch(blocklistEntry);
    expect(entry.wallet.toString()).to.equal(flaggedWallet.publicKey.toString());
  });

  it("removes a wallet from the blocklist", async () => {
    const [blocklistEntry] = PublicKey.findProgramAddressSync(
      [Buffer.from("blocklist"), mint.toBuffer(), flaggedWallet.publicKey.toBuffer()],
      program.programId,
    );
    const [hookConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("hook-config"), mint.toBuffer()],
      program.programId,
    );

    await program.methods
      .removeFromBlocklist()
      .accounts({
        blocklistEntry,
        hookConfig,
        mint,
        wallet: flaggedWallet.publicKey,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();
  });
});
