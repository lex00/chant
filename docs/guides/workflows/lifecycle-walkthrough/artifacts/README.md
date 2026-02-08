# Lifecycle Walkthrough Example

This directory contains a working example project that demonstrates the complete spec lifecycle from the [Lifecycle Walkthrough guide](../lifecycle-walkthrough.md).

## What's Inside

### Example CLI Tool: `datalog`

A minimal log analysis CLI tool in Python that serves as the foundation for the walkthrough. It provides:

- **src/datalog.py** - Simple CLI with query and analyze commands
- **src/query.py** - Query execution logic
- **src/export.py** - Export functionality (built during the walkthrough)
- **tests/test_query.py** - Basic tests
- **tests/test_export.py** - Export tests (built during the walkthrough)

### Spec Artifacts

Pre-built spec files corresponding to each phase of the walkthrough:

- **spec-001-initial.md** - Initial complex spec (Phase 1: Create)
- **spec-001-focused.md** - Simplified spec after editing (Phase 1: Create)
- **spec-001-driver.md** - Driver spec after split (Phase 2: Split)
- **spec-001.1-csv-handler.md** - CSV export handler member (Phase 2: Split)
- **spec-001.2-command-skeleton.md** - Command skeleton member (Phase 2: Split)
- **spec-001.3-integration-tests.md** - Integration tests member (Phase 2: Split)
- **spec-004-severity-field.md** - Follow-up spec for drift fix (Phase 10: React)

## Following Along

### Setup

1. Copy the artifacts directory to your working location:
   ```bash
   cp -r docs/guides/workflows/lifecycle-walkthrough/artifacts ~/datalog-tutorial
   cd ~/datalog-tutorial
   ```

2. Initialize chant in the directory:
   ```bash
   chant init
   ```

3. Install dependencies:
   ```bash
   pip install click pytest
   ```

### Following the Walkthrough

The guide walks through building the export feature. You can:

1. **Start from scratch**: Use the spec files as reference to compare your work
2. **Skip ahead**: Copy a spec file into `.chant/specs/` to jump to a specific phase
3. **Compare results**: Check your generated specs against the pre-built versions

### Running Tests

The example includes a test script that validates the setup:

```bash
./test.sh
```

This verifies:
- Chant can initialize in the example directory
- Spec files are valid and can be linted
- Source files exist and tests can run
- Drift detection works on the example specs

## What You'll Build

By following the walkthrough with this example, you'll:

1. Start with a complex spec for adding CSV/JSON export
2. Simplify it based on lint feedback
3. Split it into three focused member specs with dependencies
4. Execute the specs in chain mode with worktree isolation
5. Encounter a test failure (empty dataset edge case)
6. Recover by manually fixing and retrying
7. Verify the completed work
8. Detect drift when a new field is added
9. Create a follow-up spec to address the drift

## Key Learning Points

- **Linting catches complexity early** - The initial spec is intentionally too complex
- **Splitting creates manageable work** - Three small specs vs. one large spec
- **Dependencies ensure order** - Command needs handler, tests need both
- **Failure is part of the process** - Edge cases are discovered during testing
- **Verification detects drift** - Changes to the data model are caught automatically

## Testing the Example

Run the test script to validate the example works:

```bash
./test.sh
```

Expected output:
```
✓ chant init succeeds
✓ spec files are valid
✓ can lint spec files
✓ source files exist
✓ tests can run
✓ drift detection works
```
