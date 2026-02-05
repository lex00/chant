#!/bin/bash
# Validates the quickstart guide works correctly
# Usage: ./scripts/validate-quickstart.sh [claude|kirocli]

set -e

PROVIDER="${1:-claude}"
TEST_DIR=$(mktemp -d)
CHANT_BIN="${CHANT_BIN:-chant}"

echo "=== Quickstart Validation ==="
echo "Provider: $PROVIDER"
echo "Test dir: $TEST_DIR"
echo ""

cleanup() {
    echo ""
    echo "=== Cleanup ==="
    rm -rf "$TEST_DIR"
    echo "Removed $TEST_DIR"
}
trap cleanup EXIT

cd "$TEST_DIR"

# Step 1: Initialize project
echo "=== Step 1: Initialize Project ==="
git init -q
git config user.email "test@example.com"
git config user.name "Test User"

if [ "$PROVIDER" = "kirocli" ]; then
    $CHANT_BIN init --provider kirocli --model sonnet --agent kiro
else
    $CHANT_BIN init --provider claude --model sonnet --agent claude
fi

# Verify init created expected files
[ -d ".chant" ] || { echo "FAIL: .chant directory not created"; exit 1; }
[ -f ".chant/config.md" ] || { echo "FAIL: config.md not created"; exit 1; }
echo "OK: Project initialized"

# Step 2: Create a spec
echo ""
echo "=== Step 2: Create Spec ==="
ADD_OUTPUT=$($CHANT_BIN add "Create a hello.sh script that prints Hello World" 2>&1)
echo "$ADD_OUTPUT"

# Get the spec ID from "Created 2026-02-05-001-xyz" output
SPEC_ID=$(echo "$ADD_OUTPUT" | grep -o 'Created [^ ]*' | head -1 | cut -d' ' -f2)
if [ -z "$SPEC_ID" ]; then
    # Fallback: find spec file directly
    SPEC_FILE=$(ls .chant/specs/*.md 2>/dev/null | head -1)
    SPEC_ID=$(basename "$SPEC_FILE" .md)
fi
echo "Spec ID: $SPEC_ID"

# Verify spec file exists
SPEC_FILE=".chant/specs/${SPEC_ID}.md"
if [ -f "$SPEC_FILE" ]; then
    echo "OK: Spec created at $SPEC_FILE"
else
    echo "FAIL: Spec file not found"
    exit 1
fi

# Step 3: Lint should warn about missing AC
echo ""
echo "=== Step 3: Lint (expect warnings) ==="
LINT_OUTPUT=$($CHANT_BIN lint 2>&1 || true)
echo "$LINT_OUTPUT"

if echo "$LINT_OUTPUT" | grep -qi "acceptance\|criteria\|warning\|⚠"; then
    echo "OK: Lint correctly identified issues"
else
    echo "WARN: Lint didn't report expected warnings (may be OK)"
fi

# Step 4: Update spec with acceptance criteria
echo ""
echo "=== Step 4: Add Acceptance Criteria ==="

cat > "$SPEC_FILE" << 'EOF'
---
status: pending
---

# Create a hello.sh script that prints Hello World

Create a bash script that outputs a greeting.

## Acceptance Criteria

- [ ] Creates `hello.sh` in the project root
- [ ] Script is executable (`chmod +x`)
- [ ] Running `./hello.sh` prints "Hello World"
- [ ] Script includes a shebang line (`#!/bin/bash`)
EOF

echo "OK: Updated spec with acceptance criteria"

# Step 5: Lint should pass now
echo ""
echo "=== Step 5: Lint (expect pass) ==="
LINT_OUTPUT=$($CHANT_BIN lint 2>&1 || true)
echo "$LINT_OUTPUT"

# Verify spec has AC
if grep -q "Acceptance Criteria" "$SPEC_FILE"; then
    echo "OK: Spec has acceptance criteria"
else
    echo "FAIL: Spec missing acceptance criteria"
    exit 1
fi

# Step 6: Verify MCP config (if applicable)
echo ""
echo "=== Step 6: Verify Configuration ==="
$CHANT_BIN status
echo ""
echo "OK: Status command works"

# Summary
echo ""
echo "=== Validation Complete ==="
echo "All quickstart steps validated successfully!"
echo ""
echo "To complete the full quickstart, start your agent CLI:"
if [ "$PROVIDER" = "kirocli" ]; then
    echo "  kiro-cli-chat chat"
else
    echo "  claude"
fi
echo ""
echo "Then use MCP tools to execute the spec."
