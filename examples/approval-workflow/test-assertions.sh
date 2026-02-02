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

# Test 6: Verify README links are valid
echo -n "✓ Checking README documentation links... "
BROKEN_LINKS=0
README_DIR="$(dirname "$EXAMPLE_DIR/README.md")"
while IFS= read -r line_num link_path; do
    if [[ -z "$link_path" ]]; then continue; fi
    cd "$README_DIR"
    if [[ "$(uname)" == "Darwin" ]]; then
        target=$(python3 -c "import os; print(os.path.realpath('$link_path'))" 2>/dev/null || echo "")
    else
        target=$(realpath "$link_path" 2>/dev/null || echo "")
    fi
    cd - >/dev/null
    if [[ -z "$target" ]] || [[ ! -e "$target" ]]; then
        ((BROKEN_LINKS++))
    fi
done < <(grep -n '\[.*\](.*\.md)' README.md 2>/dev/null | sed -E 's/^([0-9]+):.*\[.*\]\(([^)#]+)(#[^)]+)?\).*/\1 \2/' || echo "")
if [[ $BROKEN_LINKS -eq 0 ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Found $BROKEN_LINKS broken link(s)"
    ((FAILED++))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
