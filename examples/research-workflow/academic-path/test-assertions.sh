#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== Research Workflow (Academic Path) Validation ==="

# Test 1: Verify research spec exists
echo -n "✓ Checking research spec exists... "
if [[ -f ".chant/specs/001-lit-review.md" ]] && \
   grep -q "type: research" ".chant/specs/001-lit-review.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing or invalid research spec"
    ((FAILED++))
fi

# Test 2: Verify informed_by references papers
echo -n "✓ Checking spec references papers... "
if grep -q "informed_by:" ".chant/specs/001-lit-review.md" && \
   (grep -q "papers/" ".chant/specs/001-lit-review.md" || grep -q "smith" ".chant/specs/001-lit-review.md"); then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing informed_by field"
    ((FAILED++))
fi

# Test 3: Verify paper source files exist
echo -n "✓ Checking paper files exist... "
if [[ -f "papers/smith2025.md" ]] && [[ -f "papers/jones2024.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing paper source files"
    ((FAILED++))
fi

# Test 4: Verify target_files specifies output location
echo -n "✓ Checking spec has target_files... "
if grep -q "target_files:" ".chant/specs/001-lit-review.md" && \
   grep -q "findings/" ".chant/specs/001-lit-review.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing target_files field"
    ((FAILED++))
fi

# Test 5: Verify acceptance criteria are structured as checkboxes
echo -n "✓ Checking spec has acceptance criteria checkboxes... "
if grep -q "## Acceptance Criteria" ".chant/specs/001-lit-review.md" && \
   grep -q "\- \[" ".chant/specs/001-lit-review.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing acceptance criteria checkboxes"
    ((FAILED++))
fi

# Test 6: Verify papers directory exists
echo -n "✓ Checking papers directory exists... "
if [[ -d "papers" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing papers directory"
    ((FAILED++))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
