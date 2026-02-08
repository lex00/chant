---
id: 2026-03-01-004-abc
type: code
status: pending
created: 2026-03-01T09:00:00Z
informed_by:
  - 2026-02-08-001-xyz
target_files:
  - src/export.py
  - tests/test_export.py
---

# Add severity field to CSV export

Add the new `severity` field to CSV export to address drift detected in spec 001.3.

## Context

The data model was updated to include a `severity` field on log entries. Verification detected that the CSV export doesn't include this field, causing partial verification failure on spec 001.3.

## Requirements

- Update `export_csv()` to include severity field in output
- Add severity to header row
- Update tests to verify severity is exported
- Ensure existing tests still pass

## Target Files

- src/export.py
- tests/test_export.py

## Acceptance Criteria

- [ ] Severity field included in CSV export
- [ ] Header row includes "severity" column
- [ ] Tests verify severity is exported
- [ ] Existing tests still pass

## Informed By

- 2026-02-08-001-xyz - Original export feature spec

## Notes

This is the follow-up spec created in Phase 10: React.
Addresses drift detected by `chant verify` in Phase 9.
After completion, spec 001.3 will verify clean again.
