# Cost Tracking

> **Status: Not Implemented** ❌
>
> Cost tracking and token usage monitoring are planned features.
> They are not currently implemented. See [Roadmap](../roadmap.md) - Phase 8 for future plans.

## Overview

Track token usage and costs per spec. Enables budgeting and optimization.

## Per-Spec Tracking

Agent reports token usage, Chant records in spec frontmatter:

```yaml
---
status: completed
usage:
  input_tokens: 12500
  output_tokens: 3200
  total_tokens: 15700
  cost_usd: 0.47
  model: provider/model-name
  duration_s: 45
---
```

## Adaptive Token Consumption

Start small, expand if needed:

```yaml
# config.md
costs:
  adaptive: true
  initial_context: 8000      # Start with 8k context
  max_context: 100000        # Can expand to 100k
  expand_threshold: 0.9      # Expand when 90% used
```

Behavior:
1. First attempt uses `initial_context` tokens
2. If spec fails or agent requests more, expand
3. Retry with larger context
4. Never exceed `max_context`

Benefits:
- Simple specs stay cheap
- Complex specs get resources they need
- No manual tuning

## Budget Limits

### Per-Spec Limits

```yaml
# config.md
costs:
  spec_limit_usd: 5.00       # Max per spec
  spec_limit_tokens: 100000  # Max tokens per spec
```

If limit reached:
```bash
$ chant work 2026-01-22-001-x7m
Error: Spec exceeded cost limit ($5.00)
  Spent: $5.23
  Tokens: 112,340

Use --override-limit to continue, or increase costs.spec_limit_usd
```

### Daily/Monthly Limits

```yaml
costs:
  daily_limit_usd: 50.00
  monthly_limit_usd: 500.00
```

```bash
$ chant work 2026-01-22-001-x7m
Error: Daily budget exhausted ($50.00)
  Spent today: $50.12

Reset at midnight UTC, or increase costs.daily_limit_usd
```

### Project Limits (Scale)

```yaml
scale:
  costs:
    auth:
      daily_limit_usd: 100.00
    payments:
      daily_limit_usd: 200.00
```

## Cost Reporting

```bash
# Today's spending
chant costs today
# Total: $23.45 (47 specs)

# This month
chant costs month
# Total: $342.12 (1,245 specs)
# By project:
#   auth:     $89.23 (312 specs)
#   payments: $156.78 (521 specs)
#   other:    $96.11 (412 specs)

# Per spec
chant costs show 2026-01-22-001-x7m
# Tokens: 15,700 (12,500 in / 3,200 out)
# Cost: $0.47
# Model: claude-3-opus
# Duration: 45s
```

## Metrics (Scale)

Prometheus metrics for cost monitoring:

```prometheus
# Total spend
chant_costs_usd_total 342.12
chant_costs_usd_total{project="auth"} 89.23

# Token usage
chant_tokens_total{type="input"} 4523000
chant_tokens_total{type="output"} 1234000

# Cost per spec (histogram)
chant_spec_cost_usd_bucket{le="0.1"} 890
chant_spec_cost_usd_bucket{le="1.0"} 1180
chant_spec_cost_usd_bucket{le="5.0"} 1240
chant_spec_cost_usd_bucket{le="+Inf"} 1245
```

## Provider Pricing

Chant tracks pricing for configured providers:

```yaml
# config.md
agent:
  provider: my-provider
  pricing:
    input: 0.01       # per 1k tokens
    output: 0.02
```

## Optimization Suggestions

```bash
chant costs optimize
```

Output:
```
Cost optimization suggestions:

1. Spec 2026-01-22-005-abc used 95k tokens for simple fix
   Suggestion: Break into smaller specs
   Potential savings: ~$2.00

2. Project "docs" averaging $1.20/spec (team avg: $0.45)
   Suggestion: Use smaller model for documentation specs
   Potential savings: ~$50/month

3. 12 specs failed then succeeded on retry
   Suggestion: Improve acceptance criteria
   Potential savings: ~$8.00 (duplicate work)
```

## No-Cost Mode

For testing without real API calls:

```yaml
# config.md
agent:
  provider: mock
  # Returns success without calling API
```

Or use local models (no API costs).

## Cost Storage

Costs stored in:

1. **Spec frontmatter** - Per-spec usage
2. **Daily log** - `.chant/costs/YYYY-MM-DD.json`
3. **Daemon** - In-memory aggregates (scale)

```json
// .chant/costs/2026-01-22.json
{
  "date": "2026-01-22",
  "total_usd": 23.45,
  "total_tokens": 523400,
  "specs": [
    {"id": "2026-01-22-001-x7m", "cost_usd": 0.47, "tokens": 15700},
    ...
  ]
}
```
