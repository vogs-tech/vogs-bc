import { Connection, Keypair, clusterApiUrl } from "@solana/web3.js";
import {
  createToken2022Mint,
  createTokenAccount,
  mintTokens,
} from "../tests/helpers/tokens";
import { loadKeypair } from "../tests/helpers/wallets";

const HOOK_PROGRAM_ID = "VHook1111111111111111111111111111111111111";

async function main() {
  const connection = new Connection(clusterApiUrl("devnet"), "confirmed");
  const authority = loadKeypair("acme");
  const muller = loadKeypair("muller");

  console.log("Creating Token-2022 mints on devnet...\n");

  // vUSDC — 6 decimals, Transfer Hook extension
  const vUSDC = await createToken2022Mint(
    connection,
    authority,
    6,
    new (await import("@solana/web3.js")).PublicKey(HOOK_PROGRAM_ID),
  );
  console.log(`vUSDC: ${vUSDC.toBase58()}`);

  // vEURC — 6 decimals
  const vEURC = await createToken2022Mint(connection, authority, 6);
  console.log(`vEURC: ${vEURC.toBase58()}`);

  // vUSDY — 6 decimals, InterestBearing extension at 482 bps APY
  const vUSDY = await createToken2022Mint(connection, authority, 6, undefined, 482);
  console.log(`vUSDY: ${vUSDY.toBase58()}`);

  // vTER — 9 decimals (tokenized gold)
  const vTER = await createToken2022Mint(connection, authority, 9);
  console.log(`vTER:  ${vTER.toBase58()}`);

  // vPAXG — 9 decimals (tokenized gold)
  const vPAXG = await createToken2022Mint(connection, authority, 9);
  console.log(`vPAXG: ${vPAXG.toBase58()}`);

  // Mint initial balances to acme and muller
  console.log("\nMinting initial balances...");

  for (const [name, mint, decimals] of [
    ["vUSDC", vUSDC, 6],
    ["vEURC", vEURC, 6],
    ["vUSDY", vUSDY, 6],
    ["vTER", vTER, 9],
    ["vPAXG", vPAXG, 9],
  ] as const) {
    const amount = decimals === 6 ? 10_000_000_000_000 : 1_000_000_000_000; // 10M or 1K

    const acmeAta = await createTokenAccount(connection, authority, mint, authority.publicKey);
    await mintTokens(connection, authority, mint, acmeAta.address, amount);
    console.log(`  ${name}: Minted ${amount} to acme`);

    const mullerAta = await createTokenAccount(connection, authority, mint, muller.publicKey);
    await mintTokens(connection, authority, mint, mullerAta.address, amount / 10);
    console.log(`  ${name}: Minted ${amount / 10} to muller`);
  }

  console.log("\nToken creation complete!");
  console.log("NOTE: flagged.json wallet receives NO tokens and should be added to blocklist.");
}

main().catch(console.error);
