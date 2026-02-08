#!/usr/bin/env bash
set -e

# Test script for KPI/OKR workflow artifacts
# Validates spec files and context documents

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "=== KPI/OKR Workflow Artifacts Validation ==="
echo

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }

# Test 1: Check spec files exist
echo -n "Checking spec files exist... "
if [ -f "research-spec-001-xyz.md" ] && \
   [ -f "driver-spec-002-abc.md" ]; then
    pass "PASS"
else
    fail "FAIL - spec files missing"
fi

# Test 2: Check context document exists
echo -n "Checking context document exists... "
if [ -f "datadog-churn-metrics-2026-01.md" ]; then
    pass "PASS"
else
    fail "FAIL - context document missing"
fi

# Test 3: Verify spec frontmatter has required fields
echo -n "Checking spec frontmatter is valid... "
spec_valid=true
for spec in research-spec-001-xyz.md driver-spec-002-abc.md; do
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

# Test 4: Verify approval workflow is present in research spec
echo -n "Checking approval workflow in research spec... "
if grep -q "^approval:" "research-spec-001-xyz.md" && \
   grep -q "REJECTED" "research-spec-001-xyz.md" && \
   grep -q "APPROVED" "research-spec-001-xyz.md"; then
    pass "PASS"
else
    fail "FAIL - approval workflow missing from research spec"
fi

echo
echo "=== All checks passed ==="
