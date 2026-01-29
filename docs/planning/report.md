# Activity Reports

**Status:** Planning

## Overview

`chant report` command for generating team analytics and activity summaries.

## Proposed Usage

```bash
# Weekly report
chant report --last 7d

# Monthly report with output format
chant report --last 30d --format json

# Report for specific project/label
chant report --last 7d --label backend
```

## Proposed Output

```
Weekly Report (2026-01-15 to 2026-01-22)

Tasks:
  Created:    47
  Completed:  42
  Failed:     5 (3 retried successfully)

By Label:
  auth:       12 completed
  payments:   18 completed
  api:        12 completed

Top Failures:
  - Test failures: 3
  - Merge conflicts: 2

Avg Duration: 12m
Total Agent Time: 8.4h
```

## Design Questions

1. What metrics are most useful for teams?
2. Should reports be cacheable/exportable?
3. How to handle multi-repo reporting?
4. Should cost tracking be included?

## Related

- Current `chant status` provides basic counts
- Git-based analytics can supplement this (see docs/scale/observability.md)
