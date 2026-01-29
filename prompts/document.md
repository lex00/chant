---
name: document
purpose: Write user-facing documentation
---

# Write Documentation

You are writing user-facing documentation for a fix or feature.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Your Goal

Create clear, helpful documentation for users:

1. **Understand what changed:**
   - Read the implementation spec (in `informed_by`)
   - Understand the new behavior
   - Note any breaking changes

2. **Update existing documentation:**
   - API reference (if methods changed)
   - User guides (if workflows changed)
   - Configuration docs (if options changed)
   - Examples (ensure they still work)

3. **Write new documentation:**
   - Migration guides for breaking changes
   - Troubleshooting entries for common issues
   - FAQ additions if relevant
   - Tutorials for new features

4. **Ensure quality:**
   - Write for users, not developers
   - Focus on what and why, not how
   - Include working code examples
   - Keep it concise but complete

## Output

Updated documentation files at the `target_files` locations with:

1. Clear explanations of new/changed behavior
2. Working code examples
3. Migration guidance (if breaking changes)
4. Troubleshooting entries (if relevant)

## Documentation Principles

### Write for Users

❌ "The write() method now acquires a mutex before the read-modify-write cycle"

✅ "The write() method is now safe to use from multiple threads simultaneously"

### Include Examples

```rust
// Before (might lose data)
store.write("key", "value")?;  // Not thread-safe

// After (safe)
store.write("key", "value")?;  // Thread-safe, no changes needed
```

### Highlight Breaking Changes

> ⚠️ **Breaking Change**
>
> `write()` now blocks if another write is in progress. If your code
> relies on non-blocking writes, use `write_async()` instead.

### Provide Migration Paths

```markdown
## Migrating from v0.6 to v0.7

### write() behavior change

If your application uses `write()` in performance-critical paths:

1. **No action needed** if occasional blocking is acceptable
2. **Use write_async()** if you need non-blocking writes
3. **Configure timeout** if you need to limit wait time
```

## Instructions

1. Read the implementation spec to understand what changed
2. Identify which documentation needs updating
3. Update existing docs with new behavior
4. Add new sections as needed
5. Verify all code examples work
6. Mark acceptance criteria as complete in `{{spec.path}}`
7. Commit with message: `chant({{spec.id}}): <description>`

## Quality Checklist

- [ ] Written from user's perspective
- [ ] All examples tested and working
- [ ] Breaking changes clearly marked
- [ ] Migration guidance provided
- [ ] No jargon without explanation
- [ ] Cross-references updated

## Constraints

- Focus on user needs, not implementation details
- Don't duplicate information unnecessarily
- Keep examples minimal but complete
- Update table of contents and navigation
- Maintain consistent style with existing docs
