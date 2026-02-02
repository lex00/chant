# Guide Evaluation: recovery.md and ecosystem.md

## Executive Summary

**Recommendation:**
- **recovery.md**: Keep as single-file reference guide (no expansion needed)
- **ecosystem.md**: Remove or consolidate into existing documentation

## Analysis

### recovery.md

**Current Coverage (178 lines):**
- Recovery philosophy
- Quick reference commands
- Failure scenarios (agent crash, machine reboot, network failure, git conflicts)
- State diagrams
- Rollback/undo procedures
- Checkpoints
- Configuration options
- Best practices

**Code References:**
- src/diagnose.rs:5
- src/merge.rs:5
- src/merge_driver.rs:37
- src/conflict.rs:8

**Documentation References:**
- docs/README.md:45 (main guide list)
- docs/SUMMARY.md:38 (TOC)
- docs/guides/oss-maintainer-workflow/index.md:221 (related workflow link)
- docs/FEATURE_STATUS.md:181 (marked as "✅ Current")
- docs/doc-audit-map.toml (tracked for 4 source files)

**Assessment:**
- **Substantial**: Yes - covers multiple complex failure scenarios with concrete examples
- **Well-referenced**: Yes - cited by source code and other documentation
- **Complete**: Yes - provides comprehensive reference for recovery operations
- **Current**: Actively maintained and updated (last audit 2026-01-25)

**Recommendation: Keep as single-file guide**

Rationale:
1. Recovery operations are inherently reference material - users need quick lookup during failures
2. Single-file format is optimal for emergency scenarios (no navigation overhead)
3. Content is comprehensive but concise enough to scan quickly
4. Already has good structure (quick reference, scenarios, config)
5. Active maintenance shows it's valuable
6. No need for multi-file expansion - would hurt usability during stressful recovery situations

### ecosystem.md

**Current Coverage (145 lines):**
- Integration philosophy
- Model repositories (basic provider adapter documentation)
- Prompt categories (theoretical taxonomy)
- Prompt composition (unimplemented features)
- Model + prompt recommendations (unimplemented)
- Template repositories (unimplemented)
- Version compatibility (unimplemented)
- Offline mode (unimplemented)
- Contributing guidelines (unimplemented)

**Implementation Status:**
- Lines 3-7: Explicit warning that most features are "Partially Implemented ⚠️"
- Only model provider adapters (Claude, Ollama, OpenAI) are implemented
- Prompt registry and package management planned for future releases

**Documentation References:**
- docs/README.md:43 (main guide list)
- docs/SUMMARY.md:37 (TOC)
- docs/FEATURE_STATUS.md:62, 77, 180 (marked as "⚠️ Partially implemented")
- docs/roadmap/roadmap.md:32, 45 (future features)

**Assessment:**
- **Substantial**: No - mostly describes unimplemented features
- **Implemented**: ~10% (only provider adapters work)
- **User value**: Low - users can't actually use most documented features
- **Confusion risk**: High - extensive documentation for non-existent features
- **Duplication**: Provider adapter info could live in reference/providers.md

**Recommendation: Remove or consolidate**

Rationale:
1. 90% of content describes unimplemented features planned for v0.4.0+
2. Creates user confusion - extensive docs suggest features exist
3. Implemented portion (provider adapters) is minimal and better suited for reference docs
4. No code references it (unlike recovery.md which has 4 source files)
5. Other docs appropriately reference it as "future work" (roadmap.md)
6. Template/prompt registry examples are premature without implementation

**Consolidation Options:**

**Option A: Remove entirely**
- Move provider adapter configuration to docs/reference/providers.md or docs/reference/configuration.md
- Keep roadmap references for planned features
- Remove from main guides list

**Option B: Reduce to stub**
- Replace with 20-line overview:
  - "Model provider adapters (see reference/providers.md)"
  - "Prompt registry: planned for v0.4.0 (see roadmap)"
  - Link to relevant reference documentation

**Recommended: Option A (Remove)**
- Current content adds no actionable information for users
- Provider configuration belongs in reference section anyway
- Roadmap already documents future plans adequately

## Comparison: Multi-File Guide Examples

The project has two multi-file guides that demonstrate when expansion is warranted:

### oss-maintainer-workflow/ (16 files)
- Complete end-to-end workflow with multiple stages
- Each stage is substantial (01-comprehension.md: ~4634 bytes, 03-root-cause.md: ~11714 bytes)
- Includes practical examples and artifacts
- Covers real, implemented workflow
- High user value for target audience

### enterprise/research-workflow/ (2 sub-guides)
- academic/ and developer/ subdirectories with 8 files each
- Detailed scenarios with concrete examples
- Multiple workflow variations for different roles
- Substantial documentation per file

**Pattern:** Multi-file guides are justified when:
1. Content is substantial (multiple phases, each with 4000+ bytes of detail)
2. Features are implemented and usable
3. Workflow requires multiple sequential stages
4. Each stage has distinct concerns requiring separate documentation
5. Includes working examples and artifacts

### Recovery comparison
- recovery.md covers multiple failure scenarios but they're all variations of the same core concern (failure recovery)
- 178 lines is substantial but not complex enough to require separation
- Users need quick reference, not deep workflow exploration
- Single-file serves emergency use case better

### Ecosystem comparison
- Most content describes unimplemented features
- Only provider adapters exist (minimal configuration)
- No multi-stage workflow to document
- No working examples to demonstrate
- Would be premature to create multi-file structure for vaporware

## Recommendations Summary

### recovery.md
**Action:** Keep as-is (single-file reference guide)
- ✅ Comprehensive coverage of implemented features
- ✅ Actively maintained and referenced by source code
- ✅ Optimal format for emergency recovery scenarios
- ❌ No expansion needed - would hurt usability

### ecosystem.md
**Action:** Remove (consolidate minimal content elsewhere)
- ❌ 90% unimplemented features create user confusion
- ❌ No code references or practical examples
- ❌ Provider adapter config belongs in reference section
- ✅ Roadmap already documents planned features
- ✅ Can be recreated later when features actually exist

### Content Preservation

From ecosystem.md, preserve:

**Move to docs/reference/providers.md or docs/reference/configuration.md:**
```markdown
## Model Providers

Chant supports multiple model providers through adapter configuration:

```yaml
# config.md
agent:
  provider: my-provider
  model: my-model
  endpoint: ${PROVIDER_ENDPOINT}
```

Supported providers: Claude (Anthropic), OpenAI, Ollama, and any OpenAI-compatible endpoint.
```

**Keep in roadmap.md (already exists):**
- Prompt registry plans
- Template/prompt package management plans

**Remove entirely:**
- Prompt composition examples (no implementation)
- Template repository lists (don't exist)
- Version compatibility specs (no registry)
- Offline caching (no prompt registry to cache)
- Contributing guidelines (no registry to contribute to)

## Follow-up Actions

If recommendations accepted:

1. **Remove ecosystem.md**
   - Delete docs/guides/ecosystem.md
   - Remove from docs/README.md:43
   - Remove from docs/SUMMARY.md:37
   - Update docs/FEATURE_STATUS.md:180 (remove entry)
   - Update docs/roadmap/roadmap.md:45 (remove reference to existing guide)

2. **Add/enhance provider configuration documentation**
   - Create or update docs/reference/providers.md
   - Document supported providers (Claude, OpenAI, Ollama)
   - Show configuration examples
   - Reference from docs/reference/configuration.md

3. **Keep recovery.md unchanged**
   - Already in good state
   - Continue normal maintenance

4. **Future work**
   - When prompt registry is implemented (v0.4.0+), consider creating ecosystem guide then
   - At that point, multi-file structure may be justified if scope is large enough
