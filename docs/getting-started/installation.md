# Installation

Chant is available through multiple installation methods. Choose the one that works best for your environment.

## Quick Install (Linux/macOS)

The fastest way to get started:

```bash
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-linux-x86_64 -o chant
chmod +x chant
sudo mv chant /usr/local/bin/
```

For macOS, download the appropriate architecture:

```bash
# Intel/x86_64
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-macos-x86_64 -o chant

# Apple Silicon (aarch64)
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-macos-aarch64 -o chant

chmod +x chant
sudo mv chant /usr/local/bin/
codesign -f -s - /usr/local/bin/chant
```

## Homebrew (macOS/Linux)

If you have Homebrew installed:

```bash
brew install lex00/tap/chant
```

## Cargo (from source)

If you have Rust and Cargo installed:

```bash
cargo install --git https://github.com/lex00/chant
```

## Download from Releases

Visit the [Releases page](https://github.com/lex00/chant/releases/latest) to download pre-built binaries for your platform:

- **Linux x86_64** - `chant-linux-x86_64`
- **macOS Intel** - `chant-macos-x86_64`
- **macOS Apple Silicon** - `chant-macos-aarch64`
- **Windows** - `chant-windows-x86_64.exe`

After downloading, make it executable and move it to your PATH:

```bash
chmod +x chant
sudo mv chant /usr/local/bin/
# On macOS, re-sign the binary to prevent SIGKILL
codesign -f -s - /usr/local/bin/chant
```

## Build from Source

To build Chant from source, you'll need Rust and Git:

```bash
git clone https://github.com/lex00/chant
cd chant
cargo build --release
```

The binary will be available at `target/release/chant`. You can then move it to your PATH:

```bash
sudo mv target/release/chant /usr/local/bin/
# On macOS, re-sign the binary to prevent SIGKILL
codesign -f -s - /usr/local/bin/chant
```

## Verify Installation

After installation, verify that chant is working:

```bash
chant --version
```

You should see the version number printed.

## Getting Started After Installation

Once installed, initialize chant in your project:

```bash
chant init
```

Then proceed to the [Quickstart](quickstart.md) guide to learn how to create and execute your first spec.

## Platform Support

| Platform | Status | Architecture | Package Manager |
|----------|--------|--------------|-----------------|
| Linux | ✅ Supported | x86_64 | Homebrew, Cargo |
| macOS | ✅ Supported | x86_64, aarch64 (Apple Silicon) | Homebrew, Cargo |
| Windows | ✅ Supported | x86_64 | Direct Download |

## Troubleshooting

### Binary not found after installation

If you get "command not found: chant" after installing, ensure that `/usr/local/bin` is in your `PATH`:

```bash
echo $PATH
```

If `/usr/local/bin` is not listed, you may need to add it to your shell configuration (`.bashrc`, `.zshrc`, etc.):

```bash
export PATH="/usr/local/bin:$PATH"
```

### Permission denied

If you get a "Permission denied" error when running chant, ensure the binary is executable:

```bash
chmod +x /usr/local/bin/chant
```

### macOS process killed with SIGKILL (exit 137)

On macOS, if chant is killed immediately after running (exit code 137), the binary needs to be code-signed. This happens when macOS strips the code signature after copying the binary. Re-sign with an ad-hoc signature:

```bash
codesign -f -s - /usr/local/bin/chant
```

This is automatically handled by Homebrew and should be done after any manual installation or binary copy operation on macOS.

### Cargo installation fails

If `cargo install --git` fails, ensure you have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then try the installation again.

## Next Steps

- Read the [Quickstart](quickstart.md) guide
- Explore the [Philosophy](philosophy.md) behind Chant
- Check out the [CLI Commands](../reference/cli.md) reference
