# Advanced Patterns

## Configuration for OSS Maintainers

### Silent Mode (Keep .chant/ Local)

If you don't want `.chant/` tracked in git (useful for personal workflow on shared repos):

```bash
# Enable silent mode for this project
chant silent

# Or enable globally for all projects
chant silent --global
```

Silent mode:
- Keeps `.chant/` out of git via `.git/info/exclude`
- Suppresses warnings about untracked spec files
- Ideal for OSS maintainers who want personal spec workflow

### Working on Fix Branches

When working on a specific issue branch instead of main:

```yaml
# .chant/config.yaml
defaults:
  branch: false        # Work directly on current branch
  main_branch: "fix/issue-123"  # Target for merges
```

Or initialize with branch mode disabled:
```bash
chant init --branch=false
```

This lets you:
- Create specs for your fix
- Work directly on your fix branch
- Merge spec work into your fix branch (not main)
