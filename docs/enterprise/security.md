# Security

## Tiers

Security concerns vary by deployment:

| Tier | Secrets | Isolation | Trust Model |
|------|---------|-----------|-------------|
| Solo | Env vars, .env | None needed | Full trust |
| Team | Env vars, vault | Git branches | Team trust |
| Scale/K8s | K8s secrets | Pods, RBAC | Zero trust |

## Solo

### Secrets

```bash
# .env (gitignored)
AGENT_API_KEY=sk-...
DATABASE_URL=postgres://...

# Load before chant
source .env && chant work 2026-01-22-001-x7m
```

### Agent Permissions

Agent runs as your user. Full filesystem access. Trust the agent or:

```yaml
# config.md - restrict agent to certain paths
agent:
  allowed_paths:
    - src/
    - tests/
  denied_paths:
    - .env
    - credentials/
```

Enforcement is advisory (agent honors, not enforced).

## Team

### Secrets

```bash
# CI/CD injects secrets
# GitHub Actions
env:
  AGENT_API_KEY: ${{ secrets.AGENT_API_KEY }}

# Or: vault integration
chant work --secrets-from vault://path/to/secrets
```

### Branch Protection

- Agents work on branches, not main
- PR review before merge
- CI validates agent output

## Scale (K8s)

Kubernetes handles most security concerns:

### Secrets

```yaml
# K8s Secret
apiVersion: v1
kind: Secret
metadata:
  name: chant-secrets
data:
  AGENT_API_KEY: base64...

# Pod spec
envFrom:
  - secretRef:
      name: chant-secrets
```

### Isolation

```yaml
# Each worker is isolated pod
# Network policies restrict egress
# Service accounts limit K8s API access
apiVersion: v1
kind: ServiceAccount
metadata:
  name: chant-worker
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
rules:
  - apiGroups: [""]
    resources: ["secrets"]
    verbs: ["get"]
    resourceNames: ["chant-secrets"]
```

### Resource Limits

```yaml
resources:
  limits:
    memory: "2Gi"
    cpu: "1"
  requests:
    memory: "512Mi"
    cpu: "250m"
```

### Network Policy

```yaml
# Restrict agent network access
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
spec:
  podSelector:
    matchLabels:
      app: chant-worker
  egress:
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
      ports:
        - port: 443  # HTTPS only (API calls)
```

## Chant Doesn't Solve

These are platform concerns, not chant concerns:

| Concern | Solution |
|---------|----------|
| Secret storage | Vault, K8s secrets, env vars |
| Network isolation | K8s network policies, firewalls |
| Audit logging | K8s audit, cloud logging |
| Identity | K8s service accounts, IAM |
| Encryption | TLS, disk encryption |

Chant integrates with platforms, doesn't replace them.

## Agent Sandboxing

### Sandbox Levels

| Level | Isolation | Use Case |
|-------|-----------|----------|
| `none` | Agent runs as user | Solo, trusted specs |
| `permissions` | Agent permission system | Default, interactive |
| `docker` | Container isolation | Untrusted, CI/CD |
| `vm` | Full VM isolation | High security |

### Level: None

Agent has full access. Use only when you trust the spec completely.

```yaml
# config.md
agent:
  sandbox: none
```

### Level: Permissions (Default)

Uses the agent's built-in permission system:

```yaml
agent:
  sandbox: permissions
  permissions:
    allow_edit: [src/**, tests/**]
    deny_edit: [.env, *.key, credentials/**]
    allow_bash: [npm test, go test, cargo test]
    deny_bash: [rm -rf, curl, wget]
    allow_network: false
```

Agent will be prompted for actions outside allowed scope.

### Level: Docker

Full container isolation:

```yaml
agent:
  sandbox: docker
  docker:
    image: chant-agent:latest
    network: none                    # No network access
    read_only: true                  # Read-only root filesystem
    volumes:
      - "${CLONE_PATH}:/workspace"   # Only clone is writable
    memory: 2G
    cpu: 1
```

**Dockerfile for sandboxed agent:**

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y git nodejs npm
# No secrets, no credentials, minimal tools
USER nobody
WORKDIR /workspace
```

### Level: VM (High Security)

For maximum isolation:

```yaml
agent:
  sandbox: vm
  vm:
    provider: firecracker           # or: qemu, virtualbox
    memory: 4G
    cpu: 2
    network: isolated
    snapshot: clean                 # Reset to clean state each spec
```

### File Access Control

```yaml
agent:
  file_access:
    # Allowlist (if set, only these paths accessible)
    allowed_paths:
      - src/
      - tests/
      - docs/

    # Denylist (always blocked)
    denied_paths:
      - .env*
      - *.key
      - *.pem
      - credentials/
      - secrets/
      - .git/config         # Prevent credential theft

    # Patterns for sensitive content
    sensitive_patterns:
      - password
      - api_key
      - secret
```

### Network Control

```yaml
agent:
  network:
    enabled: false              # No network by default

    # Or allowlist
    enabled: true
    allowed_hosts:
      - api.provider.com        # LLM API
      - registry.npmjs.org      # Package registry
      - github.com              # Git operations
    denied_hosts:
      - "*"                     # Block everything else
```

### Command Control

```yaml
agent:
  commands:
    # Allowed commands (exact or prefix)
    allowed:
      - npm test
      - npm run
      - go test
      - cargo test
      - git *

    # Blocked commands
    denied:
      - rm -rf
      - sudo *
      - curl *
      - wget *
      - nc *
      - "* > /dev/*"
      - chmod 777
```

### Secrets Access

```yaml
agent:
  secrets:
    # Which env vars agent can see
    expose:
      - AGENT_API_KEY       # Needed for LLM
      - NODE_ENV
    hide:
      - DATABASE_URL
      - AWS_*
      - GITHUB_TOKEN
```

### Enforcement

| Sandbox Level | Enforcement |
|---------------|-------------|
| `none` | Honor system (agent could ignore) |
| `permissions` | Agent enforces |
| `docker` | Kernel enforces (cgroups, namespaces) |
| `vm` | Hypervisor enforces |

For untrusted specs, use `docker` or `vm`. `permissions` is good for trusted teams.

## Audit Trail

### What's Audited

Every significant action is logged:

| Action | Logged Data |
|--------|-------------|
| Spec created | who, when, initial content hash |
| Spec started | who, when, agent config, sandbox level |
| File modified | which files, by which spec |
| Command run | command, exit code, duration |
| Spec completed | who, when, commit hash, files changed |
| Spec cancelled | who, when, reason |
| Config changed | who, when, what changed |
| Lock acquired/released | spec, PID, duration |

### Audit Log Format

```json
{
  "ts": "2026-01-22T15:30:00Z",
  "event": "spec_started",
  "spec_id": "2026-01-22-001-x7m",
  "actor": "alex",
  "agent": "provider/model-name",
  "sandbox": "docker",
  "host": "build-server-01",
  "session": "sess_abc123",
  "details": {
    "prompt": "standard",
    "target_files": ["src/auth/middleware.go"]
  }
}
```

### Audit Storage

```yaml
# config.md
audit:
  enabled: true
  file: .chant/audit/audit.log
  format: jsonl                    # jsonl | json | text
  rotate: daily
  retain: 90d                      # Compliance requirement

  # Remote shipping (enterprise)
  ship_to:
    type: s3                       # s3 | gcs | elasticsearch | splunk
    bucket: company-audit-logs
    prefix: chant/
```

### Immutability

Audit logs should be tamper-evident:

```yaml
audit:
  integrity:
    enabled: true
    algorithm: sha256
    # Each entry includes hash of previous entry
    # Chain can be verified
```

```json
{"ts":"...","event":"task_started",...,"prev_hash":"abc123","hash":"def456"}
{"ts":"...","event":"task_completed",...,"prev_hash":"def456","hash":"ghi789"}
```

### Compliance Queries

```bash
# Who worked on what
$ chant audit --actor alex --last 30d

# What happened to a spec
$ chant audit --spec 001

# All file modifications
$ chant audit --event file_modified --last 7d

# Failed attempts
$ chant audit --event spec_failed

# Export for compliance
$ chant audit --last 90d --format csv > audit-q1.csv
```

### Git as Audit

Git history is also an audit trail:

```bash
# Who changed spec files
git log --format="%ai %an: %s" -- .chant/specs/

# When was spec completed
git log -1 --format=%ai -- .chant/specs/001.md

# What changed in each commit
git log -p -- .chant/specs/001.md
```

**Advantage**: Tamper-evident (git hashes), distributed (every clone has history).

### Audit vs Logging

| Aspect | Audit | Logging |
|--------|-------|---------|
| Purpose | Compliance, forensics | Debugging, ops |
| Retention | Long (90d+) | Short (7-30d) |
| Format | Structured, immutable | Flexible |
| Access | Restricted | Team-wide |
| Content | Actions, actors | Errors, performance |

Both are needed. Audit for "who did what", logs for "what went wrong".

## Sensitive Files

Chant warns about sensitive patterns:

```yaml
# config.md
security:
  sensitive_patterns:
    - "*.env"
    - "*credentials*"
    - "*secret*"
    - "*.pem"
    - "*.key"
```

```bash
$ chant work 2026-01-22-001-x7m
Warning: Spec targets sensitive file: .env.production
Continue? [y/N]
```
