#!/usr/bin/env bash
set -e

# Test script for TDD workflow artifacts
# Validates spec files and generated test code

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "=== TDD Workflow Artifacts Validation ==="
echo

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }

# Test 1: Check spec files exist
echo -n "Checking spec files exist... "
if [ -f "test-planning-spec-001-rfn.md" ] && \
   [ -f "coverage-research-spec-001-cov.md" ] && \
   [ -f "tdd-config-template.md" ] && \
   [ -f "test-suite-driver-spec.md" ]; then
    pass "PASS"
else
    fail "FAIL - spec files missing"
fi

# Test 2: Check generated test file exists
echo -n "Checking generated test file exists... "
if [ -f "generated-test-file.py" ]; then
    pass "PASS"
else
    fail "FAIL - generated-test-file.py missing"
fi

# Test 3: Verify Python syntax of generated test
echo -n "Checking Python syntax is valid... "
if command -v python3 &> /dev/null; then
    python3 -c "import ast; ast.parse(open('generated-test-file.py').read())" 2>/dev/null
    pass "PASS"
else
    echo "SKIP (python3 not available)"
fi

# Test 4: Verify spec frontmatter has required fields
echo -n "Checking spec frontmatter is valid... "
spec_valid=true
for spec in test-planning-spec-001-rfn.md coverage-research-spec-001-cov.md test-suite-driver-spec.md; do
    if ! grep -q "^type:" "$spec" || ! grep -q "^status:" "$spec"; then
        spec_valid=false
        break
    fi
done
if $spec_valid; then
    pass "PASS"
else
    fail "FAIL - spec frontmatter invalid"
fi

echo
echo "=== All checks passed ==="
