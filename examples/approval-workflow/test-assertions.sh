#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== Approval Workflow Validation ==="

# Test 1: Verify all three specs exist
echo -n "✓ Checking spec files exist... "
if [[ -f ".chant/specs/001-risky-refactor.md" ]] && \
   [[ -f ".chant/specs/002-approved-feature.md" ]] && \
   [[ -f ".chant/specs/003-rejected-change.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing spec files"
    ((FAILED++))
fi

# Test 2: Verify pending approval spec has approval.required field
echo -n "✓ Checking pending spec has approval required... "
if grep -A2 "approval:" ".chant/specs/001-risky-refactor.md" | grep -q "required: true"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - 001-risky-refactor.md missing approval.required field"
    ((FAILED++))
fi

# Test 3: Verify approved spec has approval metadata
echo -n "✓ Checking approved spec has approval metadata... "
if grep -A5 "approval:" ".chant/specs/002-approved-feature.md" | grep -q "status: approved" && \
   grep -A5 "approval:" ".chant/specs/002-approved-feature.md" | grep -q "by:"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - 002-approved-feature.md missing approval metadata"
    ((FAILED++))
fi

# Test 4: Verify rejected spec has rejection metadata
echo -n "✓ Checking rejected spec has rejection metadata... "
if grep -A5 "approval:" ".chant/specs/003-rejected-change.md" | grep -q "status: rejected" && \
   grep -A5 "approval:" ".chant/specs/003-rejected-change.md" | grep -q "by:"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - 003-rejected-change.md missing rejection metadata"
    ((FAILED++))
fi

# Test 5: Verify config.md contains approval settings
echo -n "✓ Checking config has approval settings... "
if [[ -f ".chant/config.md" ]] && \
   grep -q "approval:" ".chant/config.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - .chant/config.md missing approval configuration"
    ((FAILED++))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
