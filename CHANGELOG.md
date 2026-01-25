# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-01-25

### Added

- **Native Ollama/OpenAI provider support**: Use local or cloud LLMs directly
  - `OllamaProvider` for local models via OpenAI-compatible API
  - `OpenaiProvider` for OpenAI API with authentication
  - Provider abstraction via `ModelProvider` trait
  - Configurable via `defaults.provider` and `providers` section in config
  - Streaming output for all providers
  - Clear error messages for connection, auth, and model issues

- **`chant init --agent`**: Generate AI assistant configuration files
  - `--agent claude` creates CLAUDE.md for Claude Code
  - `--agent cursor` creates .cursorrules for Cursor IDE
  - `--agent amazonq` creates .amazonq/rules.md for Amazon Q
  - `--agent generic` creates .ai-instructions for any assistant
  - `--agent all` creates all configuration files
  - Templates embedded in binary (no network required)

- **Silent mode for private usage**: Keep chant local-only
  - `chant init --silent` adds `.chant/` to `.git/info/exclude`
  - `--pr` blocked in silent mode (prevents revealing usage)
  - `--branch` warns in silent mode (branch names visible)
  - `chant status` shows "(silent mode)" indicator
  - `--force` flag for reinitializing
  - `--minimal` flag for config-only initialization

- **`chant version` command**: Display version and build info
  - `chant --version` and `chant version` both work
  - `--verbose` flag shows commit hash and build date

- **Homebrew tap**: Install via `brew install lex00/tap/chant`

### Fixed

- **Cross-platform CI compatibility**:
  - Use `git init -b main` for consistent branch naming
  - PathBuf comparison for Windows path separators
  - Skip Unix-specific tests on Windows
  - Replace curl subprocess with ureq HTTP library

- **Standard prompt ordering**: Format/lint now runs before commit
  - Prevents uncommitted changes from `cargo fmt` after commit
  - Added "verify git status is clean" step

- **Windows binary extension**: Fixed double `.exe.exe` in release artifacts

### Changed

- mdBook tagline updated to "Idempotent Intention"

## [0.1.1] - 2026-01-25

### Added

- **`chant delete` command**: Safely remove specs with comprehensive cleanup
  - `--force` for in-progress/completed specs
  - `--cascade` to delete driver and all members
  - `--delete-branch` to remove associated git branches
  - `--dry-run` to preview deletions
  - Automatic cleanup of log files and worktrees
  - Safety checks for dependencies and member specs

- **Markdown rendering for `chant show`**: Rich terminal output using pulldown-cmark
  - Formatted headings, bold, italic, code blocks
  - Syntax highlighting for code
  - `--no-render` flag for raw output
  - Respects `NO_COLOR` environment variable
  - Auto-detects TTY for smart rendering

- **Conflict auto-spawn**: Automatic conflict resolution spec creation
  - Detects merge conflicts during parallel execution
  - Creates detailed conflict specs with context
  - Tracks blocked specs and conflicting files
  - New `type: conflict` spec type with ⚡ indicator

- **Archive folder organization**: Date-based archive structure
  - Specs archived to `.chant/archive/YYYY-MM-DD/` folders
  - Automatic migration of flat archive files
  - `chant show` finds archived specs in subfolders

- **README badges**: CI status, license, and release badges
- **Installation documentation**: Comprehensive install guide with curl, cargo, and build instructions
- **Enhanced standard prompt**: Guidance for out-of-scope issues and duplicate prevention

### Fixed

- Release workflow now properly triggers on version tags
- `chant show` now finds archived specs
- Test failures from parallel execution interference
- Formatting issues in generated code

### Changed

- Log command now auto-follows by default (`--no-follow` to disable)
- Archive command automatically includes all group members

## [0.1.0] - 2026-01-25

### Added

- **Core Spec System**: Markdown-based spec format with YAML frontmatter for declaring work intentions
- **Spec Execution**: Agent-driven execution of specs with acceptance criteria validation
- **Isolated Worktrees**: Automatic creation and management of git worktrees for spec execution
- **Git Integration**: Seamless branch creation, merging, and commit management for each spec
- **Command-Line Interface**:
  - `chant init` - Initialize chant in a project
  - `chant add` - Create new specs
  - `chant list` - List all specs with status
  - `chant work` - Execute a spec
  - `chant show` - Display spec details
  - `chant merge` - Merge completed specs back to main branch
  - `chant status` - View project status
  - `chant diagnose` - Check spec execution health
  - `chant split` - Break specs into smaller components
  - `chant archive` - Archive completed specs
  - `chant log` - View spec execution logs
- **Spec Types**: Support for code, task, driver, and group specs
- **Driver Specs**: Coordinate execution of multiple dependent specs
- **Parallel Execution**: Run multiple ready specs in parallel with isolated worktrees
- **Configuration Management**: Global and project-level configuration with git provider support
- **Model Context Protocol (MCP)**: Server implementation for integrating with Claude and other AI models
- **Acceptance Criteria**: Checkbox-based tracking of completion requirements
- **Labels**: Tag specs for organization and filtering
- **Model Persistence**: Track which model completed each spec
- **Pull Request Creation**: Automatic PR creation with merge summaries
- **Spec Member System**: Split specs into numbered components with dependency ordering
- **Dry-Run Mode**: Preview merge operations before executing
- **Comprehensive Testing**: 227 unit tests + integration tests ensuring reliability

### Features

- **Intent-Driven Development**: Specs document intentions; agents implement them
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Auditability**: All work tracked in git with clear lineage
- **Drift Detection**: Identify when reality diverges from specifications
- **Idempotent Operations**: Specs designed for safe re-execution
- **Flexible Execution**: Branch mode (PR-based) or direct mode (commits to main)

### Technical Details

- Written in Rust with strong type safety
- Built on clap for CLI argument parsing
- Uses git2-rs for git operations
- Supports multiple git providers (GitHub, GitLab, Gitea)
- YAML parsing with serde_yaml
- Cross-platform (tested on Linux, macOS, Windows)

## Getting Started

See [README.md](README.md) for installation and quick start instructions.

For comprehensive documentation, visit [lex00.github.io/chant](https://lex00.github.io/chant).
