import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { createToken2022Mint, createTokenAccount, mintTokens } from "./helpers/tokens";
import { airdropSol } from "./helpers/wallets";

describe("vogs-streams", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VogsStreams as Program;
  const sender = Keypair.generate();
  const recipient = Keypair.generate();
  let mint: PublicKey;

  before(async () => {
    await airdropSol(provider.connection, sender);
    await airdropSol(provider.connection, recipient);
    mint = await createToken2022Mint(provider.connection, sender, 6);
  });

  it("rejects a stream with invalid time range", async () => {
    const nonce = new anchor.BN(1);
    const [streamPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("stream"), sender.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );

    const escrowAta = await createTokenAccount(provider.connection, sender, mint, streamPda);
    const senderAta = await createTokenAccount(provider.connection, sender, mint, sender.publicKey);
    await mintTokens(provider.connection, sender, mint, senderAta.address, 1_000_000);

    const now = Math.floor(Date.now() / 1000);
    try {
      await program.methods
        .createStream(new anchor.BN(1_000_000), new anchor.BN(now + 100), new anchor.BN(now), nonce)
        .accounts({
          stream: streamPda,
          mint,
          escrowTokenAccount: escrowAta.address,
          senderTokenAccount: senderAta.address,
          recipient: recipient.publicKey,
          sender: sender.publicKey,
        })
        .signers([sender])
        .rpc();
      expect.fail("Should have thrown");
    } catch (err: any) {
      expect(err.error.errorCode.code).to.equal("InvalidTimeRange");
    }
  });

  it("creates a valid stream", async () => {
    const nonce = new anchor.BN(2);
    const [streamPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("stream"), sender.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );

    const escrowAta = await createTokenAccount(provider.connection, sender, mint, streamPda);
    const senderAta = await createTokenAccount(provider.connection, sender, mint, sender.publicKey);
    await mintTokens(provider.connection, sender, mint, senderAta.address, 10_000_000);

    const now = Math.floor(Date.now() / 1000);
    await program.methods
      .createStream(
        new anchor.BN(10_000_000),
        new anchor.BN(now),
        new anchor.BN(now + 3600), // 1 hour
        nonce,
      )
      .accounts({
        stream: streamPda,
        mint,
        escrowTokenAccount: escrowAta.address,
        senderTokenAccount: senderAta.address,
        recipient: recipient.publicKey,
        sender: sender.publicKey,
      })
      .signers([sender])
      .rpc();

    const stream = await program.account.stream.fetch(streamPda);
    expect(stream.totalAmount.toNumber()).to.equal(10_000_000);
    expect(stream.status).to.deep.equal({ active: {} });
  });
});
