# Claude Code Instructions for Chant

## Primary Rule

**All work must be done through chant specs.** Do not write code, run tests, or make changes directly.

## Workflow

1. **Create a spec**: `just chant new "description of the task"`
2. **Work the spec**: `just chant work <spec-id>`

That's it. The chant system handles everything else.

## Commands

- `just chant` - Show available commands
- `just chant new "title"` - Create a new spec
- `just chant list` - List all specs
- `just chant work <id>` - Execute a spec
- `just chant show <id>` - View spec details

## What NOT to do

- Do not edit source files directly
- Do not run `cargo test` or `cargo build` directly
- Do not create TODO lists or task trackers
- Do not plan implementations outside of specs

## When asked to implement something

1. Create a spec with `just chant new`
2. Run `just chant work` on that spec
3. Report the result
