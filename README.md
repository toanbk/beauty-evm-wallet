# beauty-wallet-address

A **vanity Ethereum address generator** written in Rust. It creates random BIP39 wallets on the standard derivation path and brute-forces in parallel until the **hex address (without `0x`)** ends with your chosen suffix.

## Features

- **Derivation path**: `m/44'/60'/0'/0/0` (common Ethereum HD path)
- **Matching**: **suffix-only** match on the hex address (input is case-insensitive; normalized to lowercase)
- **Parallelism**: multi-threaded attempts via [rayon](https://github.com/rayon-rs/rayon)
- **Output**: JSON (mnemonic, address, private key, timestamp); incremental saves as matches are found
- **Safety**: result file mode `0600` on Unix; atomic write via temp file + `rename`
- **Existing output**: if the output path already exists, it is renamed with a timestamp before a new run

## Requirements and build

- [Rust](https://www.rust-lang.org/) (stable recommended, `edition = "2021"`)

```bash
cargo build --release
```

The release binary is `target/release/beauty-wallet`.

## Usage

```bash
# Find one address ending in 1988 (default count is 1)
./target/release/beauty-wallet --suffix 1988

# Find several matches
./target/release/beauty-wallet --suffix 8888 --count 3

# Run until Ctrl+C
./target/release/beauty-wallet --suffix abcd --continuous

# Custom output path and live attempt rate
./target/release/beauty-wallet --suffix a --output ./my-wallets.json --verbose
```

### CLI options

| Flag | Description |
|------|-------------|
| `-s`, `--suffix` | Hex suffix to match (required; max 40 hex characters) |
| `-c`, `--count` | Stop after this many matches (default `1`) |
| `--continuous` | Run until interrupted; cannot be used with `--count` |
| `-o`, `--output` | JSON output path (default `beauty-wallet-results.json`) |
| `-v`, `--verbose` | Show attempt count and attempts per second |

`Ctrl+C` stops gracefully; wallets found up to that point are still written to the final JSON.

## JSON output

Each entry includes:

- `mnemonic`: 12-word BIP39 mnemonic
- `address`: lowercase hex with `0x` prefix (not EIP-55 checksummed)
- `private_key`: hex private key with `0x` prefix
- `found_at`: UTC timestamp (RFC3339 string)

**Warning:** The output file is full custody of any funds sent to those addresses. Store it offline; never commit it or paste it into chat.

## Search difficulty

Each extra **hex digit** in the suffix multiplies expected attempts by roughly **16**. Long suffixes become impractical quickly.

## Tests

```bash
cargo test
```

Integration tests invoke the real CLI with short suffixes (e.g. `a`) and may take a bit longer.

## License

There is no top-level `LICENSE` in this repository yet; add one before redistribution or commercial use if you need explicit terms.

## Repository layout

This repo also contains ClaudeKit / OpenCode agent configuration (`.claude/`, `.opencode/`). It is unrelated to the `beauty-wallet` binary at runtime.
