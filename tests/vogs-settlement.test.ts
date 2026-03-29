import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { createToken2022Mint } from "./helpers/tokens";
import { airdropSol } from "./helpers/wallets";

describe("vogs-settlement", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VogsSettlement as Program;
  const authority = Keypair.generate();
  const sender = Keypair.generate();
  let sourceMint: PublicKey;
  let destMint: PublicKey;

  before(async () => {
    await airdropSol(provider.connection, authority);
    await airdropSol(provider.connection, sender);

    sourceMint = await createToken2022Mint(provider.connection, authority, 6);
    destMint = await createToken2022Mint(provider.connection, authority, 6);
  });

  it("updates oracle price", async () => {
    const [oraclePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("oracle"), sourceMint.toBuffer(), destMint.toBuffer()],
      program.programId,
    );

    await program.methods
      .updateOracle(new anchor.BN(1_120_000), 6) // 1.12 with 6 decimals
      .accounts({
        oraclePrice: oraclePda,
        sourceMint,
        destMint,
        authority: authority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    const oracle = await program.account.oraclePrice.fetch(oraclePda);
    expect(oracle.price.toNumber()).to.equal(1_120_000);
    expect(oracle.decimals).to.equal(6);
  });

  it("creates a payment with valid oracle", async () => {
    const nonce = new anchor.BN(1);
    const [paymentPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("payment"), sender.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );
    const [oraclePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("oracle"), sourceMint.toBuffer(), destMint.toBuffer()],
      program.programId,
    );

    const recipient = Keypair.generate();
    const travelRuleHash = Buffer.alloc(32);
    travelRuleHash.write("test-travel-rule-hash");

    await program.methods
      .createPayment(new anchor.BN(150_000_000_000), Array.from(travelRuleHash), nonce)
      .accounts({
        payment: paymentPda,
        oraclePrice: oraclePda,
        sourceMint,
        destMint,
        recipient: recipient.publicKey,
        sender: sender.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([sender])
      .rpc();

    const payment = await program.account.payment.fetch(paymentPda);
    expect(payment.sourceAmount.toNumber()).to.equal(150_000_000_000);
    expect(payment.status).to.deep.equal({ created: {} });
  });
});
