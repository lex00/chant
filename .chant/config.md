---
project:
  name: chant

defaults:
  prompt: standard
  branch: false
  pr: false
  model: sonnet
  rotation_strategy: round-robin

parallel:
  stagger_delay_ms: 10000
  agents:
    #- name: claude1
    #  command: claude1
    #  max_concurrent: 1
    #  weight: 1
    - name: claude2
      command: claude2
      max_concurrent: 3
      weight: 3
    - name: claude3
      command: claude3
      max_concurrent: 3
      weight: 3
  cleanup:
    enabled: true
    prompt: parallel-cleanup
    auto_run: false
---

# Chant Configuration

Project initialized on 2026-01-27.
