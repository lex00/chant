# Notifications

> **Status: Not Implemented** ❌
>
> This feature is documented as a planned enhancement for the Notifications layer.
> It is not currently implemented. See [Roadmap](../roadmap.md) for planned releases.

## Overview

Notify humans when specs complete, fail, or need attention. Notifications are markdown templates.

## Notification Templates

```
.chant/notifications/
  on_complete.md      # Spec completed
  on_fail.md          # Spec failed
  on_blocked.md       # Spec became blocked
  on_stale.md         # Spec in_progress too long
```

## Template Format

```markdown
# .chant/notifications/on_complete.md
---
channel: slack
webhook: ${SLACK_WEBHOOK_URL}
---

Spec **{{spec.id}}** completed.

{{#if branch}}
Branch: `{{branch}}`
{{/if}}

{{#if pr_url}}
PR: {{pr_url}}
{{/if}}

Duration: {{duration}}
```

## Template Variables

| Variable | Description |
|----------|-------------|
| `{{spec.id}}` | Spec ID |
| `{{spec.title}}` | First line of spec body |
| `{{status}}` | Final status |
| `{{duration}}` | Execution duration |
| `{{branch}}` | Git branch (if created) |
| `{{pr_url}}` | PR URL (if created) |
| `{{error}}` | Error message (on_fail) |
| `{{project}}` | Project prefix |
| `{{prompt}}` | Prompt used |
| `{{attempt}}` | Attempt number |

## Channels

### Webhook (Generic)

```yaml
---
channel: webhook
url: https://example.com/hook
method: POST
headers:
  Authorization: "Bearer ${TOKEN}"
---
```

Body is rendered markdown, sent as JSON:

```json
{
  "text": "Spec **2026-01-22-001-x7m** completed."
}
```

### Slack

```yaml
---
channel: slack
webhook: ${SLACK_WEBHOOK_URL}
---
```

Markdown converted to Slack mrkdwn format.

### Discord

```yaml
---
channel: discord
webhook: ${DISCORD_WEBHOOK_URL}
---
```

### Email (via webhook)

Use email service webhook (SendGrid, Mailgun):

```yaml
---
channel: webhook
url: https://api.sendgrid.com/v3/mail/send
method: POST
headers:
  Authorization: "Bearer ${SENDGRID_API_KEY}"
  Content-Type: application/json
template: |
  {
    "personalizations": [{"to": [{"email": "dev@example.com"}]}],
    "from": {"email": "chant@example.com"},
    "subject": "Spec {{spec.id}} {{status}}",
    "content": [{"type": "text/plain", "value": "{{body}}"}]
  }
---
```

### File (Local)

Write to local file (for testing, local monitoring):

```yaml
---
channel: file
path: .chant/notifications.log
---
```

## Configuration

### Global Notifications

```yaml
# config.md
notifications:
  on_complete: .chant/notifications/on_complete.md
  on_fail: .chant/notifications/on_fail.md
  on_stale:
    template: .chant/notifications/on_stale.md
    threshold: 1h   # Notify if in_progress > 1 hour
```

### Per-Spec Override

```yaml
# spec frontmatter
---
status: pending
notify:
  on_complete: slack
  on_fail: [slack, email]
---
```

### Disable

```yaml
---
status: pending
notify: false
---
```

## Triggers

| Event | Template | When |
|-------|----------|------|
| `on_complete` | on_complete.md | Spec marked completed |
| `on_fail` | on_fail.md | Spec marked failed |
| `on_blocked` | on_blocked.md | Dependency failed |
| `on_stale` | on_stale.md | in_progress too long |
| `on_retry` | on_retry.md | Retry started |

## Daemon Integration

Daemon monitors for notification triggers:

```rust
fn watch_notifications() {
    for event in spec_state_changes() {
        if event.is_completion() {
            send_notification("on_complete", &event.spec);
        } else if event.is_failure() {
            send_notification("on_fail", &event.spec);
        }
    }
}
```

Without daemon: CLI sends notifications synchronously after spec completion.

## Testing

```bash
# Test notification template
chant notify test on_complete --spec 2026-01-22-001-x7m

# Dry run (show what would be sent)
chant notify test on_complete --spec 2026-01-22-001-x7m --dry-run
```

## Example: Team Slack Integration

```markdown
# .chant/notifications/on_complete.md
---
channel: slack
webhook: ${SLACK_WEBHOOK_URL}
---

:white_check_mark: *{{spec.id}}* completed

> {{spec.title}}

{{#if pr_url}}
:link: {{pr_url}}
{{/if}}

_{{duration}} · {{prompt}} prompt_
```

```markdown
# .chant/notifications/on_fail.md
---
channel: slack
webhook: ${SLACK_WEBHOOK_URL}
---

:x: *{{spec.id}}* failed

> {{spec.title}}

```
{{error}}
```

_Attempt {{attempt}} · {{prompt}} prompt_

<@oncall> please investigate
```
