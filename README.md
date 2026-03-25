# Beauty Wallet

Ethereum vanity wallet address generator written in Rust. Brute-force generates BIP39 mnemonics on derivation path `m/44'/60'/0'/0/0` and checks for suffix matches using all CPU cores.

## Install

### From release (recommended)

```bash
# Linux x86_64
curl -L https://github.com/toanbk/beauty-evm-wallet/releases/latest/download/beauty-wallet-linux-x86_64 -o beauty-wallet
chmod +x beauty-wallet && sudo mv beauty-wallet /usr/local/bin/

# macOS Intel
curl -L https://github.com/toanbk/beauty-evm-wallet/releases/latest/download/beauty-wallet-darwin-x86_64 -o beauty-wallet
chmod +x beauty-wallet && sudo mv beauty-wallet /usr/local/bin/

# macOS Apple Silicon
curl -L https://github.com/toanbk/beauty-evm-wallet/releases/latest/download/beauty-wallet-darwin-aarch64 -o beauty-wallet
chmod +x beauty-wallet && sudo mv beauty-wallet /usr/local/bin/
```

### From source

```bash
git clone https://github.com/toanbk/beauty-evm-wallet.git
cd beauty-evm-wallet
cargo build --release
sudo cp target/release/beauty-wallet /usr/local/bin/
```

## Usage

```bash
# Find 1 wallet ending in "1988"
beauty-wallet --suffix 1988

# Find 3 wallets ending in "aa"
beauty-wallet --suffix aa --count 3

# Run continuously until Ctrl+C
beauty-wallet --suffix 8888 --continuous

# Custom output file + verbose speed stats
beauty-wallet --suffix 1988 --output my-wallets.json --verbose

# Check version
beauty-wallet --version
```

## CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `-s, --suffix` | Hex suffix to match (required, max 40 chars) | — |
| `-c, --count` | Number of wallets to find | 1 |
| `--continuous` | Run until Ctrl+C (conflicts with --count) | false |
| `-o, --output` | Output JSON file path | beauty-wallet-results.json |
| `-v, --verbose` | Show live attempts/sec stats | false |
| `--version` | Print version | — |

## JSON Output

Results are saved with 0600 permissions:

```json
[
  {
    "mnemonic": "word1 word2 ... word12",
    "address": "0x...1988",
    "private_key": "0x...",
    "found_at": "2026-03-25T00:00:00Z"
  }
]
```

If the output file already exists, it is automatically backed up with a timestamp before writing new results.

## Search Difficulty

Each extra hex digit multiplies expected attempts by 16x.

| Suffix Length | Combinations | Est. Time (16-core) |
|---------------|-------------|---------------------|
| 1 char | 16 | instant |
| 2 chars | 256 | instant |
| 4 chars | 65,536 | < 1s |
| 6 chars | 16.7M | ~2-3 min |
| 8 chars | 4.3B | ~10 hours |

## Security

- Cryptographically secure randomness (OsRng via BIP39 crate)
- Private keys only written to file, never printed to stdout
- Output file permissions: 0600 (owner read/write only)
- Atomic writes via temp file + rename (safe against Ctrl+C)
- Standard BIP39 derivation path: `m/44'/60'/0'/0/0`

**Warning:** The output file contains full custody keys. Store it securely, never commit it to git.

## Tests

```bash
cargo test
```

13 tests: 7 unit (including known BIP39 vector test) + 6 integration.

## License

MIT
