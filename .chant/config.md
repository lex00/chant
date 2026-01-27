---
project:
  name: chant

defaults:
  prompt: standard
  branch: false
  pr: false
  #model: qwen2.5:7b
  model: haiku
  #provider: ollama
  split_model: sonnet

parallel:
  agents:
    - name: claude1
      command: claude1
      max_concurrent: 1    # limited - often has active session
    - name: claude2
      command: claude2
      max_concurrent: 3
    - name: claude3
      command: claude3
      max_concurrent: 3
  cleanup:
      enabled: true
      prompt: parallel-cleanup
      auto_run: false
---

# Chant Configuration

Project initialized on 2026-01-24.
