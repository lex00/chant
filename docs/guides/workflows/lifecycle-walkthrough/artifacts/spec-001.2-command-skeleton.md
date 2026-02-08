---
id: 2026-02-08-001-xyz.2
type: code
status: pending
parent: 2026-02-08-001-xyz
depends_on:
  - 2026-02-08-001-xyz.1
created: 2026-02-08T10:30:00Z
target_files:
  - src/datalog.py
---

# Implement export command skeleton

Add the `export` subcommand to the datalog CLI that uses the CSV handler.

## Context

Part of the export feature implementation. This command integrates the CSV handler into the CLI interface.

## Requirements

- Add `export` subcommand to CLI using click
- Accept query pattern argument
- Accept `--format csv` flag (default to csv)
- Accept `--output` flag for file path (default to stdout)
- Call `export_csv()` with query results
- Write output to file or stdout

## Target Files

- src/datalog.py

## Acceptance Criteria

- [ ] Export subcommand exists in CLI
- [ ] --format flag accepted (csv only for now)
- [ ] --output flag writes to file
- [ ] No --output flag writes to stdout
- [ ] Command calls export_csv() correctly

## Dependencies

Depends on 001.1 (CSV handler must exist first)

## Notes

This is member spec 001.2 after splitting (Phase 2: Split).
Blocked until 001.1 completes (Phase 3: Dependencies).
Will be executed second in Phase 4: Execute.
