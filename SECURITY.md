# Security Policy

## Reporting a Vulnerability

We take security issues in Chant seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

1. **GitHub Security Advisories** (preferred): Use the [Security Advisory](https://github.com/lex00/chant/security/advisories/new) page to privately report vulnerabilities
2. **Email**: Send details to the repository maintainers (check repository for contact information)

### What to Include

Please include the following information in your report:

- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if available)
- Impact of the vulnerability, including how an attacker might exploit it

### Response Timeline

- **Initial Response**: We will acknowledge your report within 48 hours
- **Status Update**: We will provide a more detailed response within 7 days, indicating next steps
- **Fix Timeline**: We aim to release fixes for verified vulnerabilities within 30 days when possible
- **Disclosure**: We will coordinate with you on public disclosure timing

### Security Update Process

When a security vulnerability is confirmed:

1. We will develop and test a fix
2. We will prepare a security advisory
3. We will release a new version with the fix
4. We will publish the security advisory with credit to the reporter (unless anonymity is requested)

## Scope

This security policy applies to:

- The Chant CLI tool
- The Chant library (lib.rs)
- The MCP (Model Context Protocol) server integration
- All supported platforms (Linux, macOS, Windows)

## Security Considerations

When using Chant, please be aware:

- **Spec Execution**: Chant executes specs that may contain arbitrary commands. Only execute specs from trusted sources
- **Git Integration**: Chant integrates with git and may create branches, commits, and worktrees. Ensure your repository is backed up
- **File System Access**: Chant reads and writes files in your project directory. Review spec contents before execution
- **AI Agent Integration**: Chant integrates with AI agents (Claude). Be mindful of sensitive data in specs and code

## Supported Versions

Security updates are provided for:

- The latest stable release
- The previous stable release (when possible)

Older versions may not receive security updates. We recommend staying on the latest stable release.

## Best Practices

To use Chant securely:

1. **Review Specs**: Always review spec contents before running `chant work`
2. **Trusted Sources**: Only execute specs from trusted sources
3. **Sensitive Data**: Avoid including secrets, credentials, or sensitive data in specs
4. **Permissions**: Run Chant with appropriate user permissions (avoid running as root)
5. **Updates**: Keep Chant updated to the latest version
6. **Isolation**: Use Chant's isolated worktree feature to test untrusted specs

## Security Features

Chant includes several security-focused features:

- **Isolated Worktrees**: Specs execute in isolated git worktrees, preventing interference with your main branch
- **Approval Workflow**: The approval mode (`--approval`) allows human review before spec execution
- **Read-Only Verification**: The `chant verify` command checks spec status without modification

## Acknowledgments

We appreciate the security research community's efforts in responsibly disclosing vulnerabilities. Security researchers who report valid vulnerabilities will be acknowledged in security advisories (unless they request anonymity).
