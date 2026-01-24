# Storage & Indexing

## Directory Structure

```
.chant/
├── config.md             # Project config (git-tracked)
├── prompts/              # Prompt files (git-tracked)
│   ├── standard.md
│   ├── minimal.md
│   └── tdd.md
├── specs/                # All specs (git-tracked)
│   ├── 2026-01-22-001-x7m.md
│   └── 2026-01-22-002-q2n.md
├── .locks/               # PID files (gitignored)
└── .store/               # Index cache (gitignored)
    └── tantivy/
```

## No Archive

Specs stay in `specs/` forever. Completed specs have `status: completed`.

Why no archive folder:
- Git history preserves everything
- Moving files changes IDs (breaks references)
- Simpler mental model
- Search works on all specs regardless

Filter by status instead:
```bash
chant list                  # Active (pending, in_progress, failed)
chant list --all            # Everything
chant list --completed      # Just completed
```

## Active Specs: In-Memory

For <50 active specs, parse files in parallel on each CLI invocation.

```rust
fn load_active_specs(dir: &Path) -> Vec<Spec> {
    glob(&dir.join("*.md"))
        .par_iter()
        .filter(|f| !is_child_spec(f))  // Skip .N.md files
        .map(|f| parse_spec(f))
        .filter(|s| s.status != "completed")
        .collect()
}
```

Expected performance: ~50-100ms for 50 specs.

No persistent index needed for active specs.

## Full-Text Search: Tantivy

Search across all specs (including completed) using Tantivy.

```rust
fn build_schema() -> Schema {
    let mut builder = Schema::builder();
    builder.add_text_field("id", STRING | STORED);
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("body", TEXT);
    builder.add_text_field("labels", TEXT);
    builder.add_text_field("status", STRING);
    builder.add_date_field("completed_at", INDEXED | STORED);
    builder.build()
}
```

Index stored in `.chant/.store/tantivy/`, gitignored, rebuilt on demand.

## Change Detection

Use git to detect changed files:

```rust
fn changed_since_index() -> Vec<String> {
    Command::new("git")
        .args(["diff", "--name-only", &last_indexed_commit, "HEAD", "--", ".chant/specs/"])
        .output()
}
```

Only reindex changed files. Efficient for large spec counts.

## Scale Expectations

| Spec Count | Active Index | Search Index |
|------------|--------------|--------------|
| <50 | In-memory | Tantivy |
| 50-500 | In-memory | Tantivy |
| >500 | Consider daemon | Tantivy |

In-memory parsing scales well. Tantivy handles thousands of documents easily.

## Gitignored State

```gitignore
# .chant/.gitignore
.locks/
.store/
```

Local state is never committed:
- `.locks/` - PID files for running specs
- `.store/` - Search index cache

Both can be regenerated from spec files.
