---
name: bug-fix
description: Template for bug fix specs
type: code
labels:
  - bug-fix
variables:
  - name: bug_description
    description: Brief description of the bug
    required: true
---

# Bug Fix: {{bug_description}}

## Problem

{{bug_description}}

## Acceptance Criteria

- [ ] Bug is identified and root cause is understood
- [ ] Fix is implemented
- [ ] Tests are added to prevent regression
- [ ] Fix is verified
