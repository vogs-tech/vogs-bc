import { Connection, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";

export function loadKeypair(name: string): Keypair {
  const keyPath = path.join(__dirname, "..", "..", "keys", `${name}.json`);
  const secretKey = JSON.parse(fs.readFileSync(keyPath, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(secretKey));
}

export async function airdropSol(
  connection: Connection,
  keypair: Keypair,
  amount: number = 2,
): Promise<void> {
  const sig = await connection.requestAirdrop(
    keypair.publicKey,
    amount * LAMPORTS_PER_SOL,
  );
  await connection.confirmTransaction(sig, "confirmed");
}

export const ACME_WALLET = loadKeypair("acme");
export const MULLER_WALLET = loadKeypair("muller");
export const FLAGGED_WALLET = loadKeypair("flagged");
