# Lifecycle Walkthrough — Reference Implementation

Reference artifacts for the [Lifecycle Walkthrough](../../lifecycle-walkthrough.md) guide. These show concrete examples of what each lifecycle phase produces.

## Contents

### Source Code: `datalog` CLI

A minimal Python log analysis tool used as the scenario in the walkthrough:

| File | Purpose |
|------|---------|
| `src/datalog.py` | CLI entry point (click-based, query + analyze commands) |
| `src/query.py` | Query execution with regex pattern matching |
| `src/export.py` | Export stub (intentionally incomplete — built during walkthrough) |
| `tests/test_query.py` | Working tests for query functionality |
| `tests/test_export.py` | Export test stubs (demonstrate failure/recovery phases) |

### Spec Artifacts

Pre-built specs corresponding to lifecycle phases:

| File | Phase | What it shows |
|------|-------|---------------|
| `spec-001-initial.md` | 1: Create | Overly complex spec that triggers lint warnings |
| `spec-001-focused.md` | 1: Create | Simplified after editing based on lint feedback |
| `spec-001-driver.md` | 2: Split | Driver coordinating three members |
| `spec-001.1-csv-handler.md` | 2: Split | Member with no dependencies (executes first) |
| `spec-001.2-command-skeleton.md` | 2: Split | Member depending on 001.1 |
| `spec-001.3-integration-tests.md` | 2: Split | Member depending on 001.1 and 001.2 |
| `spec-004-severity-field.md` | 10: React | Follow-up spec with `informed_by` linking |

### Validation

`test.sh` validates the artifacts are intact:

```bash
./test.sh
```

Checks: source files exist, test files exist, spec artifacts exist, Python syntax valid, spec frontmatter has required fields. Optionally tests chant init, lint, and pytest if available.

## Dependencies

```bash
pip install -r requirements.txt
```

Requires `click` (for the CLI) and `pytest` (for tests).
