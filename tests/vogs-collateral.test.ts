import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { createToken2022Mint, createTokenAccount, mintTokens } from "./helpers/tokens";
import { airdropSol } from "./helpers/wallets";

describe("vogs-collateral", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VogsCollateral as Program;
  const owner = Keypair.generate();
  let collateralMint: PublicKey;
  let creditMint: PublicKey;

  before(async () => {
    await airdropSol(provider.connection, owner);
    collateralMint = await createToken2022Mint(provider.connection, owner, 9); // vTER gold 9 decimals
    creditMint = await createToken2022Mint(provider.connection, owner, 6); // vUSDC 6 decimals
  });

  it("creates a collateral position", async () => {
    const nonce = new anchor.BN(1);
    const [positionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        owner.publicKey.toBuffer(),
        collateralMint.toBuffer(),
        nonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId,
    );

    const collateralEscrow = await createTokenAccount(
      provider.connection,
      owner,
      collateralMint,
      positionPda,
    );
    const ownerAta = await createTokenAccount(
      provider.connection,
      owner,
      collateralMint,
      owner.publicKey,
    );
    await mintTokens(
      provider.connection,
      owner,
      collateralMint,
      ownerAta.address,
      100_000_000_000, // 100 tokens with 9 decimals
    );

    // Create a mock Pyth price account (gold at $3050/oz)
    const pythPrice = Keypair.generate();
    const priceData = Buffer.alloc(8);
    priceData.writeBigUInt64LE(BigInt(3050_000_000)); // $3050 with 6 decimals

    await program.methods
      .createPosition(
        new anchor.BN(100_000_000_000), // 100 tokens
        7500, // 75% max LTV
        8000, // 80% margin call
        9000, // 90% liquidation
        nonce,
      )
      .accounts({
        position: positionPda,
        collateralMint,
        creditMint,
        collateralEscrow: collateralEscrow.address,
        ownerCollateralAta: ownerAta.address,
        pythPriceAccount: pythPrice.publicKey,
        owner: owner.publicKey,
      })
      .signers([owner])
      .rpc();

    const position = await program.account.collateralPosition.fetch(positionPda);
    expect(position.collateralAmount.toNumber()).to.equal(100_000_000_000);
    expect(position.creditDrawn.toNumber()).to.equal(0);
    expect(position.maxLtvBps).to.equal(7500);
    expect(position.status).to.deep.equal({ active: {} });
  });
});
