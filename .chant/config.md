---
project:
  name: chant
defaults:
  prompt: standard
  branch: false
  model: sonnet
  rotation_strategy: round_robin
  provider: claude
  prompt_extensions:
  - output-concise
parallel:
  stagger_delay_ms: 3000
  agents:
  - name: claude2
    command: claude2
    max_concurrent: 2
    weight: 1
  - name: claude3
    command: claude3
    max_concurrent: 2
    weight: 1
  cleanup:
    enabled: true
    prompt: parallel-cleanup
    auto_run: false
silent: true
---
# Chant Configuration
