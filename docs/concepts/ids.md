# Spec IDs

## Filename is the ID

No separate ID field. The filename (without `.md`) is the identifier.

```
.chant/specs/2026-01-22-001-x7m.md
            └────────┬────────┘
                   ID
```

Referenced as:
- `depends_on: [2026-01-22-001-x7m]`
- `chant work 2026-01-22-001-x7m`
- `chant(2026-01-22-001-x7m): Add authentication`

## Format

### Base Format

```
YYYY-MM-DD-SSS-XXX
└────┬────┘ └┬┘ └┬┘
   date    seq  random
```

| Component | Purpose |
|-----------|---------|
| `YYYY-MM-DD` | Creation date, sortable |
| `SSS` | Sequence within day (base36: 001-zzz) |
| `XXX` | Random suffix (3 chars, base36) |

Example: `2026-01-22-001-x7m`

### Sequence Format (Base36)

Sequence uses base36 for scalability:
- `001` through `999` - familiar numeric
- `a00` through `zzz` - extends to 46,656/day

Most users never see beyond `999`. Research workflows at massive scale can exceed it.

### Project-Prefixed Format (Scale)

For monorepos with multiple projects (prefixed to spec ID):

```
PROJECT-YYYY-MM-DD-NNN-XXX
└──┬──┘ └────┬────┘ └┬┘ └┬┘
 prefix    date    seq  random
```

Example: `auth-2026-01-22-001-x7m`

**Config:**
```yaml
# config.md
project:
  prefix: auth    # Explicit prefix
```

**Or auto-detect from path:**
```yaml
scale:
  id_prefix:
    from: path
    pattern: "packages/([^/]+)/"   # packages/auth/ → auth-
```

Project prefix is optional. Solo/team deployments use base format.

## Why This Format

### Scannable
Glance at a directory listing, immediately see chronology:
```
2026-01-20-001-a3k.md
2026-01-20-002-f8p.md
2026-01-22-001-x7m.md   ← today
2026-01-22-002-q2n.md   ← today
```

### Sortable
`ls` and `find` return chronological order by default.

### Parallel-Safe
Random suffix prevents collision when:
- Multiple agents create specs simultaneously
- Multiple humans create specs on same day
- Distributed teams across timezones

### Stable
ID never changes. Title can change freely:
```yaml
---
# No id field needed - filename is the ID
status: pending
---

# Add authentication   ← Can rename this anytime
```

## Typeability

Not optimized for typing. That's fine.

- Tab completion handles long IDs
- CLI can offer fuzzy matching: `chant work 22-001` → matches `2026-01-22-001-x7m`
- Agents write most references anyway
- Copy-paste works

## Group Members

Members append `.N` suffix to driver ID:

```
2026-01-22-001-x7m.md       ← Driver
2026-01-22-001-x7m.1.md     ← Member 1
2026-01-22-001-x7m.2.md     ← Member 2
2026-01-22-001-x7m.2.1.md   ← Nested member
```

See [groups.md](groups.md) for group semantics.

## ID Generation

```rust
fn generate_id() -> String {
    let date = Local::now().format("%Y-%m-%d");
    let seq = next_sequence_for_date(&date);  // 1, 2, ... 46656
    let rand = random_base36(3);              // x7m, a3k, ...
    format!("{}-{}-{}", date, format_base36(seq, 3), rand)
}

fn format_base36(n: u32, width: usize) -> String {
    // 0 → "000", 999 → "999", 1000 → "a00", 46655 → "zzz"
    let chars = "0123456789abcdefghijklmnopqrstuvwxyz";
    // ... base36 encoding with zero-padding
}
```

Sequence resets daily. Random suffix ensures uniqueness even if sequence logic fails.

## Short ID Resolution

CLI accepts partial IDs with smart resolution:

```bash
chant work 001           # Today's 001 (if exists)
chant work 22-001        # Jan 22's 001 (this year)
chant work 01-22-001     # Jan 22's 001 (explicit month)
chant work x7m           # Suffix match (globally unique)
```

Resolution order:
1. **Exact match** - Full ID matches exactly
2. **Suffix match** - `x7m` uniquely identifies across all time
3. **Today first** - `001` checks today before searching history
4. **Date-qualified** - `22-001` or `01-22-001` for specific dates
5. **Prompt on ambiguity** - Multiple matches shows selection

The random suffix is the true unique identifier. Use it for archived specs:
```bash
chant show x7m           # Works even for old specs
```
