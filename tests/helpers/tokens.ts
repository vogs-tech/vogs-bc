import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_2022_PROGRAM_ID,
  ExtensionType,
  createInitializeTransferHookInstruction,
  createInitializeMintInstruction,
  getMintLen,
  createInitializeInterestBearingMintInstruction,
} from "@solana/spl-token";

export async function createToken2022Mint(
  connection: Connection,
  payer: Keypair,
  decimals: number,
  transferHookProgramId?: PublicKey,
  interestRateBps?: number,
): Promise<PublicKey> {
  const mintKeypair = Keypair.generate();
  const extensions: ExtensionType[] = [];

  if (transferHookProgramId) extensions.push(ExtensionType.TransferHook);
  if (interestRateBps !== undefined) extensions.push(ExtensionType.InterestBearingConfig);

  const mintLen = getMintLen(extensions);
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: mintKeypair.publicKey,
    space: mintLen,
    lamports,
    programId: TOKEN_2022_PROGRAM_ID,
  });

  const instructions = [createAccountIx];

  if (transferHookProgramId) {
    instructions.push(
      createInitializeTransferHookInstruction(
        mintKeypair.publicKey,
        payer.publicKey,
        transferHookProgramId,
        TOKEN_2022_PROGRAM_ID,
      ),
    );
  }

  if (interestRateBps !== undefined) {
    instructions.push(
      createInitializeInterestBearingMintInstruction(
        mintKeypair.publicKey,
        payer.publicKey,
        interestRateBps,
        TOKEN_2022_PROGRAM_ID,
      ),
    );
  }

  instructions.push(
    createInitializeMintInstruction(
      mintKeypair.publicKey,
      decimals,
      payer.publicKey,
      payer.publicKey,
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  const { blockhash } = await connection.getLatestBlockhash();
  const tx = new (await import("@solana/web3.js")).Transaction();
  tx.recentBlockhash = blockhash;
  tx.feePayer = payer.publicKey;
  instructions.forEach((ix) => tx.add(ix));
  tx.sign(payer, mintKeypair);

  await connection.sendRawTransaction(tx.serialize());

  return mintKeypair.publicKey;
}

export async function createTokenAccount(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey,
) {
  return getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    owner,
    false,
    undefined,
    undefined,
    TOKEN_2022_PROGRAM_ID,
  );
}

export async function mintTokens(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  destination: PublicKey,
  amount: number,
) {
  return mintTo(
    connection,
    payer,
    mint,
    destination,
    payer,
    amount,
    [],
    undefined,
    TOKEN_2022_PROGRAM_ID,
  );
}
