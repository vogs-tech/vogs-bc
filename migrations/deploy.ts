import * as anchor from "@coral-xyz/anchor";

module.exports = async function (provider: anchor.AnchorProvider) {
  anchor.setProvider(provider);

  console.log("Deploying Vogs programs to devnet...");
  console.log("Provider wallet:", provider.wallet.publicKey.toBase58());

  // Programs are deployed via `anchor deploy`
  // This migration script runs post-deployment setup

  console.log("\nPost-deployment setup:");
  console.log("1. Run: bun scripts/create-tokens.ts");
  console.log("2. Initialize hook config for vUSDC mint");
  console.log("3. Add flagged wallet to blocklist");
  console.log("4. Update program IDs in Anchor.toml");

  console.log("\nMigration complete.");
};
