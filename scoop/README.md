# Scoop Manifest

This directory contains the Scoop package manifest for Chant.

## Installation

To install Chant via Scoop:

```powershell
scoop bucket add lex00 https://github.com/lex00/scoop-bucket
scoop install chant
```

## Manifest Location

The official Scoop manifest is published to the [lex00/scoop-bucket](https://github.com/lex00/scoop-bucket) repository and is automatically updated by the release workflow in `.github/workflows/release.yml`.

The `chant.json` file in this directory serves as a reference template for the Scoop manifest structure.

## Updating the Manifest

The manifest is automatically updated when a new release is tagged. The release workflow:

1. Extracts the version from the git tag
2. Downloads the Windows binary from the release
3. Computes the SHA256 hash
4. Updates `chant.json` in the scoop-bucket repository
5. Commits and pushes the changes

No manual intervention is required for releases.
