# vogs-bc — Solana Smart Contracts

Anchor workspace with 5 Solana programs for the Vogs treasury platform: KYC-enforced transfers, yield vaults, cross-border settlement, payment streaming, and RWA collateral credit lines.

## Programs

| Program | Description |
|---------|-------------|
| vogs-hook | Transfer Hook for KYC enforcement via Civic Pass + wallet blocklist |
| vogs-vault | Permissioned yield vaults with multi-protocol allocation |
| vogs-settlement | Cross-border FX settlement with on-chain oracle pricing |
| vogs-streams | Per-second payment streaming + milestone-based escrow |
| vogs-collateral | RWA-backed collateral positions and credit lines |

## Prerequisites

- Rust 1.87+ (edition 2024)
- Anchor CLI 0.32
- Solana CLI 2.x
- Bun 1.2+ (for tests)

## Installation

```bash
bun install
```

## Configuration

Copy and edit the environment file:

```bash
cp .env.example .env
```

Key variables:
- `ANCHOR_PROVIDER_URL` — Solana cluster RPC (default: devnet)
- `ANCHOR_WALLET` — Path to keypair file

Program IDs in `Anchor.toml` are placeholders — update after `anchor keys list`.

## Build

```bash
# Build all programs
anchor build

# Build a single program
anchor build -p vogs-hook
```

## Test

```bash
anchor test
```

Tests run against a local validator. Test files are in `tests/` with shared helpers in `tests/helpers/`.

## Deploy

```bash
# Deploy to devnet
anchor deploy --provider.cluster devnet

# Create mock tokens and mint initial balances
bun scripts/create-tokens.ts
```

## Project Structure

```
vogs-bc/
├── Anchor.toml
├── Cargo.toml              # Workspace root
├── programs/
│   ├── vogs-hook/src/      # lib.rs, state.rs, errors.rs
│   ├── vogs-vault/src/     # lib.rs, state.rs, errors.rs
│   ├── vogs-settlement/src/# lib.rs, state.rs, errors.rs, events.rs
│   ├── vogs-streams/src/   # lib.rs, state.rs, errors.rs
│   └── vogs-collateral/src/# lib.rs, state.rs, errors.rs, events.rs
├── tests/                  # TypeScript integration tests
├── scripts/                # Token creation scripts
├── migrations/             # deploy.ts
└── keys/                   # Test wallet keypairs
```

## Mock Tokens

| Token | Decimals | Extensions |
|-------|----------|-----------|
| vUSDC | 6 | Transfer Hook (KYC) |
| vEURC | 6 | — |
| vUSDY | 6 | Interest Bearing (482 bps) |
| vTER | 9 | — |
| vPAXG | 9 | — |

## Test Wallets

- `keys/acme.json` — Institution wallet
- `keys/muller.json` — Recipient wallet
- `keys/flagged.json` — Blocked wallet (receives no tokens)
