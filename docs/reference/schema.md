# Schema & Validation

## The "Messy Markdown" Problem

Criticism: Markdown is messy, JSONL is clean.

Counter: Messiness is a validation problem, not a format problem. Agents can lint.

## Validation Layers

### 1. Schema Definition

Config defines required fields and valid values:

```yaml
# In config.md frontmatter
schema:
  spec:
    required: [status]    # id comes from filename
    fields:
      status:
        type: string
        enum: [pending, in_progress, completed, failed]
      depends_on:
        type: array
        items: string
      labels:
        type: array
        items: string
```

### 2. Lint on Write

Prompt instructs agent to validate before commit:

```markdown
# In prompt file
Before committing, verify the spec file:
- [ ] Frontmatter has required fields: id, status
- [ ] Status is one of: pending, in_progress, completed, failed
- [ ] All depends_on IDs exist
- [ ] YAML is valid
```

### 3. Lint on Read

Parser validates and normalizes:

```rust
fn parse_spec(path: &Path) -> Result<Spec, ValidationError> {
    let content = read_file(path)?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    let spec: Spec = serde_yaml::from_str(&frontmatter)?;

    // Validate
    validate_required(&spec)?;
    validate_status(&spec)?;
    validate_deps_exist(&spec)?;

    Ok(spec)
}
```

### 4. Pre-commit Hook

CLI provides lint command:

```bash
chant lint                    # Lint all specs
chant lint 2026-01-22-001-x7m            # Lint specific spec
chant lint --fix              # Auto-fix where possible
```

## Auto-Fix Capabilities

| Issue | Auto-fixable | Fix |
|-------|--------------|-----|
| Missing `status` | Yes | Default to `pending` |
| Invalid `status` value | No | Error, human decides |
| Missing `id` | Yes | Generate from filename |
| Trailing whitespace | Yes | Trim |
| Inconsistent indentation | Yes | Normalize to 2 spaces |
| Missing newline at EOF | Yes | Add newline |

## Validation Errors

```bash
$ chant lint
2026-01-22-001-x7m.md:
  error: status "open" not in enum [pending, in_progress, completed, failed]
  error: depends_on "2026-01-22-999-zzz" does not exist

2026-01-22-002-q2n.md:
  warning: missing optional field "labels"

2 errors, 1 warning
```

## Agent-Friendly Validation

Agents get structured feedback:

```bash
$ chant lint --json
{
  "valid": false,
  "errors": [
    {
      "file": "2026-01-22-001-x7m.md",
      "field": "status",
      "message": "value 'open' not in enum",
      "allowed": ["pending", "in_progress", "completed", "failed"]
    }
  ]
}
```

Agent can then fix and retry.

## Why This Works

1. **Agents write most specs** - they follow the prompt, which includes validation
2. **Humans can still edit** - lint catches mistakes before commit
3. **Parse errors are rare** - YAML frontmatter is simple, well-supported
4. **Recovery is easy** - fix the text file, re-run lint

The format is human-readable AND machine-validatable. Chant chooses human-first with machine validation.

## Linter as Coach

### Philosophy

The linter isn't just validation - it's **continuous guidance toward better practices**.

```
Traditional linter: "You have an error. Fix it."
Chant linter:       "Here's how to make this spec more effective."
```

### Feedback Levels

| Level | Purpose | Blocks? |
|-------|---------|---------|
| **Error** | Must fix (broken) | Yes |
| **Warning** | Should fix (risky) | Optional |
| **Suggestion** | Could improve (better) | No |
| **Tip** | Learning opportunity | No |

### Example Output

```bash
$ chant lint 001
Spec 001: Add authentication

ERRORS (must fix):
  None ✓

WARNINGS (should fix):
  ⚠ No acceptance criteria
    Specs with criteria are 3x more likely to complete correctly.
    Add a "## Acceptance Criteria" section with checkboxes.

SUGGESTIONS (could improve):
  💡 Consider adding target_files
     Helps detect drift and prevents conflicts in parallel work.
     Example: target_files: [src/auth/middleware.go]

  💡 This spec might benefit from splitting
     Large scope ("authentication") often works better as smaller tasks.
     Try: chant split 001

TIPS (learning):
  📚 You've completed 10 specs! Consider trying autonomous mode.
     See: chant help autonomy

Overall: Good spec, 2 suggestions to improve
```

### Context-Aware Guidance

The linter adapts to experience level:

```yaml
# Tracked in .chant/.state/user.json
{
  "specs_created": 47,
  "specs_completed": 42,
  "autonomy_level": "supervised",
  "features_used": ["deps", "labels"],
  "features_not_used": ["target_files", "groups", "autonomous"]
}
```

**New user (< 10 specs):**
```
```
💡 Tip: Specs work best with clear acceptance criteria.
   Example:
   ## Acceptance Criteria
   - [ ] Login returns JWT
   - [ ] Invalid credentials return 401
```

```
💡 Suggestion: You haven't tried parallel execution yet.
   Add depends_on to enable: chant work --parallel
   See: chant help deps

**Intermediate (10-50 specs):**
```

**Experienced (50+ specs):**
```
💡 Suggestion: Your completion rate is high (95%).
   Consider moving to autonomous mode for low-risk specs.
   See: chant help autonomy
```

### Spec Quality Scoring

```bash
$ chant lint 001 --score
Spec 001: Add authentication

Quality Score: 72/100

Breakdown:
  ✓ Clear title (10/10)
  ✓ Has description (15/15)
  △ No acceptance criteria (0/20) — add for +20
  ✓ Reasonable scope (15/15)
  △ No target_files (0/10) — add for +10
  ✓ Valid frontmatter (10/10)
  △ Could use labels (0/5) — add for +5
  ✓ No drift risk (15/15)

Suggested improvements:
  1. Add acceptance criteria (+20 points)
  2. Add target_files (+10 points)
  3. Add labels (+5 points)

Potential score: 97/100
```

### Project Health Dashboard

```bash
$ chant health
Project Health Report

Spec Quality:
  Average score: 78/100 (Good)
  Specs below 50: 3 (need attention)
  Specs above 90: 12 (excellent)

Practice Adoption:
  ✓ Acceptance criteria: 85% of specs
  ✓ Dependencies: 60% of specs
  △ Target files: 30% of specs (try adding more)
  △ Labels: 45% of specs
  ✗ Autonomous mode: 0% (consider trying)

Drift Status:
  Verified: 42 specs
  Drifted: 2 specs
  Unchecked: 5 specs

Recommendations:
  1. Add target_files to improve drift detection
  2. Try autonomous mode for trivial specs
  3. Review the 2 drifted tasks: 023, 089
```

### Suggestions by Category

**Spec Design:**
```
💡 Vague title: "Fix stuff"
   Better: "Fix login redirect after password reset"
   Specific titles help agents understand scope.

💡 Large scope detected
   This spec mentions 5+ components.
   Consider: chant split 001

💡 Missing context
   No description beyond title.
   Add "## Context" explaining why this matters.
```

**Autonomous Readiness:**
```
💡 This spec is a good candidate for autonomous execution
   - Clear acceptance criteria ✓
   - Small scope ✓
   - Has target_files ✓
   - Tests exist ✓
   Try: labels: [autonomous]

💡 You've been supervised for 30 specs
   Your success rate is 95%.
   Consider: autonomy.level: trusted
```

**Drift Prevention:**
```
💡 No target_files specified
   Without target_files, drift detection is limited.
   Add: target_files: [src/auth/handler.go]

💡 No acceptance criteria
   Criteria enable verification over time.
   Add checkboxes that can be re-checked.

💡 Spec completed 30 days ago, never verified
   Run: chant verify 001
```

**Collaboration:**
```
💡 Spec has no labels
   Labels help with filtering and assignment.
   Common labels: feature, bug, refactor, docs

💡 Large spec with no members
   Consider breaking down for parallel work.
   Try: chant split 001

💡 Circular dependency detected
   001 depends on 002, which depends on 001.
   Review dependency chain.
```

### Suggestion Actions

Suggestions include actionable commands:

```bash
$ chant lint 001 --suggest
💡 Add acceptance criteria

  Current:
  ---
  status: pending
  ---
  # Add authentication

  Suggested:
  ---
  status: pending
  ---
  # Add authentication

  ## Acceptance Criteria
  - [ ] <describe what "done" means>
  - [ ] <another criterion>

  Apply? [y/N/edit]
```

```bash
$ chant lint 001 --fix-suggestions
Applied 3 suggestions to spec 001:
  ✓ Added acceptance criteria section
  ✓ Added target_files from git history
  ✓ Added labels based on file paths

Review changes? [y/N]
```

### Learning Path Integration

```bash
$ chant learn
Your Chant Learning Path

Completed:
  ✓ Create first spec
  ✓ Complete first spec
  ✓ Use acceptance criteria
  ✓ Use dependencies

Next steps:
  → Try target_files for better drift detection
    Hint: Your next spec might benefit from this.

  → Enable autonomous mode for a trivial spec
    You have 3 tasks labeled "trivial" that could go autonomous.

  → Set up continuous verification
    Run: chant verify --schedule daily

Progress: ████████░░ 80% to "Chant Expert"
```

### Non-Judgmental Tone

The linter is encouraging, not critical:

```
# Bad (judgmental):
❌ Error: Your spec is poorly written.
❌ Warning: You forgot acceptance criteria again.

# Good (encouraging):
💡 Suggestion: Adding acceptance criteria helps agents succeed.
   Specs with criteria complete correctly 3x more often.

💡 Tip: You're making great progress!
   Consider trying autonomous mode for your next trivial spec.
```

### Configuration

```yaml
# config.md
lint:
  # What to show
  levels: [error, warning, suggestion]  # Exclude tips if too noisy

  # Context
  experience_tracking: true

  # Automation
  on_create: true              # Lint when spec created
  on_commit: true              # Lint in pre-commit hook
  block_on: [error]            # Only errors block

  # Scoring
  quality_scores: true
  score_threshold: 50          # Warn if spec below 50

  # Learning
  learning_path: true
  tips: true
```

### Linter Rules

Built-in rules with rationale:

```yaml
# .chant/lint-rules.md (viewable)
rules:
  - id: missing-criteria
    level: warning
    message: "No acceptance criteria"
    rationale: "Specs with criteria complete correctly 3x more often"
    autofix: true

  - id: vague-title
    level: suggestion
    pattern: "^(Fix|Update|Change) (stuff|things|it)$"
    message: "Title is vague"
    rationale: "Specific titles help agents understand scope"

  - id: large-scope
    level: suggestion
    condition: "body.word_count > 500 or body.mentions_files > 5"
    message: "Consider breaking down this spec"
    rationale: "Smaller specs complete faster and more reliably"
    action: "chant split {spec_id}"
```

### Custom Rules

Teams can add project-specific guidance:

```yaml
# config.md
lint:
  custom_rules:
    - id: security-review
      condition: "labels.includes('security') and not labels.includes('reviewed')"
      level: warning
      message: "Security tasks require review label"

    - id: payment-approval
      condition: "target_files.any(f => f.includes('payment'))"
      level: warning
      message: "Payment changes need explicit approval"
```
