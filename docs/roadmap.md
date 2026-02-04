# Roadmap

This document outlines planned features and improvements for Chant.

## Current Version: 0.15.2

Chant is under active development. We prioritize features that enhance spec-driven workflows, improve agent orchestration, and provide better visibility into AI-assisted development.

---

## Planned Features

### Short Term (Next 2-3 Releases)

#### Enhanced Observability
- **Execution metrics dashboard**: Track agent performance, token usage, and completion rates
- **Real-time progress streaming**: Live updates during spec execution with detailed task breakdowns
- **Cost analysis**: Per-spec and per-project cost tracking for LLM API calls

#### Improved Agent Control
- **Retry strategies**: Configurable retry behavior for failed specs with exponential backoff
- **Execution timeout policies**: Per-spec and global timeout controls
- **Agent temperature control**: Fine-tune model creativity per spec type

#### Workflow Enhancements
- **Spec templates**: Reusable templates for common workflows (bug fix, feature, refactor)
- **Interactive spec creation**: Guided prompts for creating well-structured specs
- **Bulk spec operations**: Create multiple related specs from a single command

### Medium Term (Next 6 Months)

#### Advanced Orchestration
- **Conditional dependencies**: Specs that execute only if conditions are met
- **Dynamic dependency resolution**: Dependencies inferred from spec content
- **Spec priorities**: High/medium/low priority queuing for execution

#### Collaboration Features
- **Spec sharing**: Export/import specs between projects
- **Team workflows**: Multi-user spec assignment and coordination
- **Review workflows**: Approval gates with reviewer assignment

#### Integration & Extensibility
- **Plugin system**: Extend chant with custom commands and workflows
- **IDE integrations**: VSCode, IntelliJ, and Neovim plugins
- **CI/CD pipelines**: GitHub Actions, GitLab CI, and CircleCI templates
- **Webhooks**: Trigger external workflows on spec events

#### Enhanced Research Workflows
- **Knowledge graph**: Link specs to create navigable project knowledge
- **Research artifact versioning**: Track evolution of research outputs
- **Citation management**: Auto-cite specs in generated documentation

### Long Term (Future Roadmap)

#### AI Capabilities
- **Multi-agent collaboration**: Specs executed by coordinated agent teams
- **Self-healing specs**: Agents automatically adjust specs based on execution feedback
- **Predictive planning**: AI-suggested spec creation based on project patterns

#### Enterprise Features
- **RBAC and audit logs**: Role-based access control with comprehensive auditing
- **Custom LLM providers**: Bring your own models and hosting
- **Air-gapped operation**: Offline mode with local models

#### Developer Experience
- **Web UI**: Browser-based spec management and execution monitoring
- **Mobile companion**: Monitor spec execution on mobile devices
- **Voice commands**: Create and execute specs via voice interface

---

## How to Influence the Roadmap

We welcome feedback and feature requests from the community:

- **GitHub Issues**: Open a feature request at [github.com/lex00/chant/issues](https://github.com/lex00/chant/issues)
- **Discussions**: Join conversations at [github.com/lex00/chant/discussions](https://github.com/lex00/chant/discussions)
- **Discord**: Connect with the community (coming soon)

Priority is given to features that:
1. Improve agent reliability and performance
2. Enhance visibility into AI-assisted work
3. Support team collaboration
4. Reduce friction in spec-driven workflows

---

## Recently Completed

These features were recently added to Chant:

- **Progress bars with indicatif** (0.14.0): Visual progress tracking during spec execution
- **Codecov integration** (0.14.0): Automated code coverage reporting
- **Docker support** (0.14.0): Container images published to GitHub Container Registry
- **MCP server integration**: Protocol support for agent communication
- **Approval workflow**: Human-in-the-loop gates for spec execution
- **Branch mode**: Isolated feature branch execution with auto-merge
- **Parallel execution**: Concurrent spec processing with isolated worktrees
- **Chain execution**: Sequential spec processing with dependency handling

See [CHANGELOG.md](../CHANGELOG.md) for full release history.

---

## Version History

- **0.15.2** (Current): Latest improvements and bug fixes
- **0.14.0**: Progress visualization, Docker, Windows support
- **0.13.x**: Approval workflow, branch mode
- **0.12.x**: Parallel execution, worktree isolation
- **0.11.x**: MCP server integration
- **0.10.x**: Core spec execution platform

---

*Last updated: 2026-02-03*
