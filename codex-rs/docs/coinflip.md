# CoinFlip Game

This module contains a Solana Anchor-based on-chain program and a Rust CLI client for a coin-flip betting game.
Users can stake the BTOP token (address: `4uqbBUTY5iT1DYue16dQisqRgbUg92fvMvfsT1tEbonk`) to play a coin flip with near-zero transaction friction via Helius RPC.

## Program

- Written in Anchor, the program defines:
  - `initialize`: Set up the game state and vault PDA.
  - `deposit`: Owner deposits liquidity into the vault.
  - `flip`: Users bet tokens on heads or tails; winners receive 2× their stake.
  - `withdraw`: Owner can withdraw unused tokens.

### Defaults

- Program ID: `<PROGRAM_ID>` (update in `Anchor.toml` after deployment).
- PDAs for state and vault use seeds `["state"]` and `["vault"]`.

## CLI

The `coinflip-cli` binary supports:

```bash
coinflip-cli \
  --rpc-url $HELIUS_RPC_URL \
  --program-id <PROGRAM_ID> \
  --keypair ~/.config/solana/id.json \
  <COMMAND>
```

Commands:

- `init --mint <MINT_ADDRESS>`: Initialize the program and vault (owner only).
- `deposit --amount <U64>`: Deposit BTOP tokens into vault (owner only).
- `flip --amount <U64> --side [heads|tails]`: Play coin flip by staking tokens.
- `withdraw --amount <U64>`: Withdraw tokens from vault (owner only).

Ensure `$HELIUS_RPC_URL` is set to your Helius RPC endpoint and `$COINFLIP_PROGRAM_ID` to the deployed program ID.

Token mint: `4uqbBUTY5iT1DYue16dQisqRgbUg92fvMvfsT1tEbonk`.