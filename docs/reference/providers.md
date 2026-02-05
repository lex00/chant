# Provider Configuration

Chant supports multiple AI model providers. Configure providers in `.chant/config.md` to control which model APIs chant uses for spec execution.

## Supported Providers

### Claude (Default)

Uses the Anthropic Claude CLI (`claude` command).

**Best for:**
- Full feature support
- Claude-specific capabilities
- Interactive development

**Requirements:**
- `claude` command installed and available in PATH
- Anthropic API key configured

**Installation:**
```bash
pip install anthropic-cli
```

**Configuration:**
```yaml
defaults:
  provider: claude
  model: claude-opus-4-5
```

No additional endpoint configuration needed.

### Ollama

OpenAI-compatible API for running local models.

**Best for:**
- Local execution
- Offline development
- Cost control
- Privacy-sensitive workloads

**Requirements:**
- Ollama running locally or accessible via network
- Supported models: Llama, Mistral, and other open-source models

**Installation:**
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Start Ollama
ollama serve

# Pull a model
ollama pull llama2
```

**Configuration:**
```yaml
defaults:
  provider: ollama
  model: llama2

providers:
  ollama:
    endpoint: http://localhost:11434/v1
```

**Remote Ollama:**
```yaml
providers:
  ollama:
    endpoint: http://ollama-server.example.com:11434/v1
```

### OpenAI

OpenAI API (GPT-4, GPT-3.5, etc.)

**Best for:**
- Production deployments
- Advanced reasoning tasks
- GPT-specific capabilities

**Requirements:**
- OpenAI API key (`OPENAI_API_KEY` environment variable)
- API access

**Configuration:**
```yaml
defaults:
  provider: openai
  model: gpt-4

providers:
  openai:
    endpoint: https://api.openai.com/v1
```

**Environment setup:**
```bash
export OPENAI_API_KEY=sk-...
```

**Azure OpenAI:**
```yaml
defaults:
  provider: openai
  model: gpt-4

providers:
  openai:
    endpoint: https://your-instance.openai.azure.com/openai
```

### Kiro CLI

Kiro CLI (`kiro-cli-chat`) for MCP-based agent execution.

**Best for:**
- MCP server integration
- Teams using Kiro ecosystem

**Requirements:**
- `kiro-cli-chat` command installed and available in PATH
- MCP servers configured via `kiro-cli-chat mcp add`

**Installation:**
```bash
# See https://kiro.dev/docs/cli for installation
```

**Configuration:**
```yaml
defaults:
  provider: kirocli
  model: sonnet  # shorthand names: sonnet, opus, haiku
```

**MCP Server Setup:**
Kiro CLI requires MCP servers to be configured separately:
```bash
# Add chant MCP server (command and args must be separate)
kiro-cli-chat mcp add --name chant --command "$(which chant)" --args mcp --scope global

# Verify configuration
kiro-cli-chat mcp list

# Should show under global:
#   • chant        /path/to/chant
```

**Verify tools are available:**
```bash
# Start interactive chat
kiro-cli-chat chat

# In the chat, type /tools to see available tools
# Should list: chant_spec_list, chant_status, chant_add, etc.
```

**Note:** Uses `kiro-cli-chat chat --no-interactive --trust-all-tools --model <model>` for automated execution.

## Configuration Reference

### Basic Provider Setup

Set the default provider in `.chant/config.md`:

```markdown
# .chant/config.md
---
project:
  name: my-project

defaults:
  provider: claude    # claude | ollama | openai | kirocli
  model: claude-opus-4-5
---
```

### Provider Endpoints

Configure provider-specific endpoints in the `providers` section:

```markdown
---
defaults:
  provider: ollama
  model: llama2

providers:
  ollama:
    endpoint: http://localhost:11434/v1
  openai:
    endpoint: https://api.openai.com/v1
---
```

### Model Selection

Specify models for each provider:

**Claude:**
```yaml
defaults:
  provider: claude
  model: claude-opus-4-5
```

**Ollama:**
```yaml
defaults:
  provider: ollama
  model: llama2          # or: mistral, mixtral, etc.
```

**OpenAI:**
```yaml
defaults:
  provider: openai
  model: gpt-4           # or: gpt-3.5-turbo, etc.
```

### Split Model Configuration

For `chant split` operations, specify a separate model:

```yaml
defaults:
  provider: ollama
  model: llama2
  split_model: mistral   # Used for split operations
```

If `split_model` is not specified, it defaults to `sonnet` (for Claude).

## Per-Spec Provider Override

Override the provider for specific specs using frontmatter:

```markdown
---
status: pending
provider: openai
model: gpt-4
---

# Spec title

This spec will use GPT-4 instead of the default provider.
```

## Global vs. Project Configuration

**Global config** (`~/.config/chant/config.md`):
```yaml
defaults:
  provider: claude
  model: claude-opus-4-5

providers:
  openai:
    endpoint: https://api.openai.com/v1
```

**Project config** (`.chant/config.md`):
```yaml
project:
  name: my-project

defaults:
  provider: ollama      # Override global default
  model: llama2
```

Project settings override global settings.

## Validation

Verify your provider configuration:

```bash
chant config --validate
```

This checks:
- Provider is supported
- Required fields are present
- Endpoint URLs are valid
- Model names are specified

## See Also

- [Configuration Reference](config.md) - Full configuration schema
- [CLI Reference](cli.md) - Command reference
