# Agent Invocation

## The Spec IS the Prompt

Specs are granular, have acceptance criteria, and are linted. The agent prompt is simple:

```
Implement this spec.

{spec file contents}
```

That's it. The spec file contains everything the agent needs:
- Title (what to do)
- Description (context)
- Acceptance criteria (definition of done)
- Target files (where to look)

## Invocation

```rust
fn invoke_agent(spec_path: &Path, prompt_path: &Path) -> Result<()> {
    let spec = read_to_string(spec_path)?;
    let prompt_content = read_to_string(prompt_path)?;

    // Template substitution
    let message = prompt_content
        .replace("{{spec}}", &spec)
        .replace("{{spec.id}}", &spec_id_from_path(spec_path));

    // Shell out to agent
    Command::new("agent")
        .arg("--print")
        .arg(&message)
        .status()?;

    Ok(())
}
```

## Default Prompt

The default prompt is minimal:

```markdown
# .chant/prompts/standard.md
---
name: standard
---

Implement this spec. Follow the acceptance criteria exactly.

When complete, commit with message: `chant({{spec.id}}): <description>`

---

{{spec}}
```

The prompt adds discipline (commit format). The spec provides specifics.

## Why Simple?

1. **Specs are well-specified** - Acceptance criteria define done
2. **Specs are linted** - Schema ensures completeness
3. **Agents are capable** - Modern AI agents can figure out the rest
4. **Less is more** - Verbose prompts add noise, not value

## Custom Prompts

Teams can add ceremony if needed:

```markdown
# .chant/prompts/tdd.md
---
name: tdd
---

Implement this spec using TDD:

1. Write failing test first
2. Implement minimum code to pass
3. Refactor if needed
4. Commit

{{spec}}
```

But the default should be minimal.

## No Orchestration in Prompt

The prompt doesn't tell the agent about:
- Other specs
- Dependencies
- Group relationships (driver/members)
- Parallel execution

That's the CLI's job. Agent focuses on one spec.

## Output Capture

Agent output goes to:
1. Terminal (streamed)
2. Spec file `progress` field (appended)

```rust
fn invoke_with_capture(spec_path: &Path, message: &str) -> Result<()> {
    let mut child = Command::new("claude")
        .arg("--print")
        .arg(message)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        println!("{}", line);                    // Terminal
        append_progress(spec_path, &line)?;      // Spec file
    }

    Ok(())
}
```
