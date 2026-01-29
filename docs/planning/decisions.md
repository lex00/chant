# Decisions Frontmatter Field

**Status:** Planning / Investigation Needed

## Overview

A `decisions:` frontmatter field for tracking design decisions made during spec implementation.

## Proposed Format

```yaml
---
type: code
status: completed
decisions:
  - Use JWT for auth tokens (simpler than OAuth for MVP)
  - Store sessions in Redis (already in our stack)
---
```

## Use Cases

1. Document trade-offs made during implementation
2. Provide context for future maintainers
3. Enable decision auditing across specs

## Design Questions

1. Should decisions be structured (key-value) or free-form strings?
2. Should there be a template for decision format (context, decision, consequences)?
3. How should decisions be rendered in `chant show`?
4. Should decisions be searchable via `chant search`?

## Investigation Needed

Check if any current specs use a `decisions:` field and how they're used. The field may already exist in some form.

## Related

- Architecture decision records (ADRs) as an alternative pattern
- Spec body can already contain decision rationale in prose
