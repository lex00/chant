#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== Research Workflow (Developer Path) Validation ==="

# Test 1: Verify research spec exists
echo -n "✓ Checking research spec exists... "
if [[ -f ".chant/specs/001-tech-debt.md" ]] && \
   grep -q "type: research" ".chant/specs/001-tech-debt.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing or invalid research spec"
    ((FAILED++))
fi

# Test 2: Verify informed_by references code
echo -n "✓ Checking spec references code... "
if grep -q "informed_by:" ".chant/specs/001-tech-debt.md" && \
   (grep -q "src/" ".chant/specs/001-tech-debt.md" || grep -q "sample-code" ".chant/specs/001-tech-debt.md"); then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing informed_by field"
    ((FAILED++))
fi

# Test 3: Verify source code files exist
echo -n "✓ Checking source code files exist... "
if [[ -f "src/sample-code/auth.rs" ]] && \
   [[ -f "src/sample-code/database.rs" ]] && \
   [[ -f "src/sample-code/utils.rs" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing source code files"
    ((FAILED++))
fi

# Test 4: Verify target_files specifies output location
echo -n "✓ Checking spec has target_files... "
if grep -q "target_files:" ".chant/specs/001-tech-debt.md" && \
   grep -q "analysis/" ".chant/specs/001-tech-debt.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing target_files field"
    ((FAILED++))
fi

# Test 5: Verify acceptance criteria are structured as checkboxes
echo -n "✓ Checking spec has acceptance criteria checkboxes... "
if grep -q "## Acceptance Criteria" ".chant/specs/001-tech-debt.md" && \
   grep -q "\- \[" ".chant/specs/001-tech-debt.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing acceptance criteria checkboxes"
    ((FAILED++))
fi

# Test 6: Verify src directory exists
echo -n "✓ Checking src directory exists... "
if [[ -d "src/sample-code" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing src/sample-code directory"
    ((FAILED++))
fi

# Test 7: Verify README links are valid
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
