# Troubleshooting

Common issues with chant and their solutions.

## Installation & Setup

### macOS code signing: `cargo install` builds get killed

**Symptom:** After running `cargo install chant`, the binary gets killed when you try to run it.

**Cause:** macOS code signing verification fails for unsigned binaries built from source.

**Fix:**
```bash
codesign -f -s - $(which chant)
```

### `chant init` fails in non-git repo

**Symptom:** Running `chant init` produces an error about not being in a git repository.

**Cause:** Chant requires a git repository to track spec history and branches.

**Fix:**
```bash
git init
git commit --allow-empty -m "Initial commit"
chant init
```

### Shell completion not loading

**Symptom:** Tab completion doesn't work for `chant` commands.

**Cause:** Shell completion script not installed or not sourced in your shell config.

**Fix:**

For bash:
```bash
chant completions bash >> ~/.bashrc
source ~/.bashrc
```

For zsh:
```bash
chant completions zsh >> ~/.zshrc
source ~/.zshrc
```

For fish:
```bash
chant completions fish > ~/.config/fish/completions/chant.fish
```

## Spec Execution

### Agent exits with code 137 (SIGKILL)

**Symptom:** `chant work` fails with exit code 137, or agent process disappears without error.

**Cause:** System killed the process due to memory pressure. This often happens when too many Claude processes are running in parallel.

**Fix:**
```yaml
# config.md
parallel:
  max_workers: 2  # Reduce from default
```

Or run specs sequentially:
```bash
chant work <id>  # Don't use --parallel
```

### Spec stuck in `in_progress` with no running agent

**Symptom:** `chant list` shows a spec as `in_progress`, but no agent is actually running.

**Cause:** Agent crashed or was killed without cleaning up its lock file.

**Fix:**

Check if an agent is actually running:
```bash
chant diagnose <id>
```

If no agent is running, either take over or reset:
```bash
# Take over to analyze partial work
chant takeover <id>

# Or reset to start fresh
chant reset <id>
```

### Chain mode stops after first spec

**Symptom:** Running `chant work <id> --chain` completes one spec but doesn't continue to the next ready spec.

**Cause:** Known issue with finalization not triggering chain continuation in some cases.

**Fix:**

Check if the work actually completed:
```bash
chant list
chant diagnose <id>
```

If spec is completed, manually start the next one:
```bash
chant ready  # Find next ready spec
chant work <next-id> --chain
```

### `chant work` fails with "not a git repository" in worktree

**Symptom:** Agent starts but immediately fails with git-related errors, or can't find `.git` directory.

**Cause:** Stale worktree from previous run that wasn't cleaned up properly.

**Fix:**
```bash
chant cleanup  # Remove stale worktrees
```

Or manually:
```bash
rm -rf .chant/.worktrees/<spec-id>
git worktree prune
```

## Parallel Mode

### Specs fail immediately with no log/branch/error

**Symptom:** When running `chant work --parallel`, some specs transition to `failed` instantly with no error message, log file, or git branch.

**Cause:** Race condition in worktree creation (fixed in chant 0.18.2+).

**Fix:**

Upgrade to chant 0.18.2 or later:
```bash
cargo install chant --force
```

Or run specs sequentially instead:
```bash
chant work <id>  # Without --parallel
```

### API rate limiting errors in parallel mode

**Symptom:** Multiple specs fail with rate limit errors when running in parallel.

**Cause:** Too many concurrent API requests to Claude API.

**Fix:**
```yaml
# config.md
parallel:
  max_workers: 3  # Reduce from default (5)
```

Or add delays between worker starts:
```yaml
parallel:
  stagger_delay: 5s  # Wait 5s between starting workers
```

## Git & Merge

### Merge conflicts in spec frontmatter

**Symptom:** Git merge conflicts in `.chant/specs/*.md` files, particularly in the YAML frontmatter.

**Cause:** Multiple specs modifying the same spec file simultaneously (common in parallel mode).

**Fix:**

Install the chant merge driver:
```bash
chant init --merge-driver
```

This configures git to use chant's custom merge strategy for spec files.

If conflicts still occur, resolve manually:
```bash
git mergetool
# Or edit the file directly, keeping the most recent status/metadata
```

### `git mv` fails for `.chant/` files

**Symptom:** Running `git mv` on files in `.chant/` directory produces an error.

**Cause:** Some files in `.chant/` (like logs, status files) are not tracked by git.

**Fix:**

Use regular `mv` instead:
```bash
mv .chant/specs/old-id.md .chant/specs/new-id.md
git add .chant/specs/new-id.md
git rm .chant/specs/old-id.md
```

### Archive fails

**Symptom:** `chant archive <partial-id>` fails to find the spec or produces an error.

**Cause:** Ambiguous partial ID matching multiple specs, or trying to archive non-completed spec.

**Fix:**

Use the full spec ID:
```bash
chant archive 2026-02-07-00x-x5v  # Full ID, not just "00x"
```

Verify spec is completed first:
```bash
chant list | grep <id>
```

## Watch & Recovery

### Watch not detecting completed specs

**Symptom:** `chant watch` is running, but doesn't detect when a spec completes in its worktree.

**Cause:** Spec didn't write `.chant-status.json` in the worktree, or watch process isn't polling.

**Fix:**

Check if status file exists:
```bash
cat .chant/.worktrees/<spec-id>/.chant-status.json
```

If missing, the agent may have crashed. Check logs:
```bash
chant log <spec-id>
```

Restart watch:
```bash
chant watch stop
chant watch start
```

### Stale worktrees accumulating

**Symptom:** `.chant/.worktrees/` directory grows large with old worktree directories.

**Cause:** Worktrees not cleaned up after specs complete or fail.

**Fix:**
```bash
chant cleanup  # Remove stale worktrees
```

Or enable automatic cleanup:
```yaml
# config.md
worktrees:
  auto_cleanup: true
  cleanup_delay: 1h  # Clean up 1 hour after completion
```

## Debugging Steps

### How to check agent logs

Agent logs are written to `.chant/logs/<spec-id>.log`:

```bash
# View full log
cat .chant/logs/2026-02-07-00x-x5v.log

# View last 50 lines
tail -n 50 .chant/logs/2026-02-07-00x-x5v.log

# Follow log in real-time
tail -f .chant/logs/2026-02-07-00x-x5v.log

# Or use chant command
chant log 00x --lines 50
```

### How to check spec status

```bash
# List all specs with status
chant list

# Get summary
chant status

# Diagnose specific spec
chant diagnose <spec-id>
```

The `diagnose` command checks:
- Spec file validity
- Lock file status
- Log file existence and activity
- Git branch status
- Acceptance criteria

### How to inspect worktree state

```bash
# List all worktrees
git worktree list

# Check status in specific worktree
git -C .chant/.worktrees/<spec-id> status

# View worktree commits
git -C .chant/.worktrees/<spec-id> log
```

### How to recover from broken state

For a spec stuck in a bad state:

1. **Diagnose the issue:**
   ```bash
   chant diagnose <spec-id>
   ```

2. **If agent is running but stuck:**
   ```bash
   chant takeover <spec-id>  # Analyze partial work
   ```

3. **If agent crashed:**
   ```bash
   chant reset <spec-id>  # Clear state and start fresh
   ```

4. **If worktree is corrupted:**
   ```bash
   chant cleanup
   chant reset <spec-id>
   ```

5. **If spec file is corrupted:**
   ```bash
   # Edit manually
   vim .chant/specs/<spec-id>.md

   # Validate
   chant diagnose <spec-id>
   ```

For general repository issues:
```bash
# Clean up all stale state
chant cleanup

# Verify all completed specs
chant verify --all

# Check for recovery-needed specs
chant recover --check
```
