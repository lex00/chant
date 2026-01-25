# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
