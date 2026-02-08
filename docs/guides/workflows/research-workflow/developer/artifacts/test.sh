#!/usr/bin/env bash
set -e

# Test script for research workflow (developer path) artifacts
# Validates spec files for microservices migration research

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "=== Research Workflow (Developer) Artifacts Validation ==="
echo

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }

# Test 1: Check spec files exist
echo -n "Checking spec files exist... "
if [ -f "coupling-analysis-spec.md" ] && \
   [ -f "architecture-docs-spec.md" ] && \
   [ -f "extraction-driver-spec.md" ] && \
   [ -f "weekly-coupling-spec.md" ]; then
    pass "PASS"
else
    fail "FAIL - spec files missing"
fi

# Test 2: Verify spec frontmatter has required fields
echo -n "Checking spec frontmatter is valid... "
spec_valid=true
for spec in coupling-analysis-spec.md architecture-docs-spec.md extraction-driver-spec.md weekly-coupling-spec.md; do
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

# Test 3: Verify dependency chain exists
echo -n "Checking dependency chain in specs... "
if grep -q "depends_on:" "architecture-docs-spec.md"; then
    pass "PASS"
else
    fail "FAIL - dependency chain missing"
fi

# Test 4: Verify driver spec has members
echo -n "Checking driver spec structure... "
if grep -q "^type: driver" "extraction-driver-spec.md"; then
    pass "PASS"
else
    fail "FAIL - driver spec missing type: driver"
fi

echo
echo "=== All checks passed ==="
