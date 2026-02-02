#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== TDD Workflow Validation ==="

# Test 1: Verify coverage analysis research spec exists
echo -n "✓ Checking coverage analysis spec exists... "
if [[ -f ".chant/specs/001-coverage-analysis.md" ]] && \
   grep -q "type: research" ".chant/specs/001-coverage-analysis.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing or invalid coverage analysis spec"
    ((FAILED++))
fi

# Test 2: Verify driver spec exists with members
echo -n "✓ Checking driver spec has members... "
if [[ -f ".chant/specs/002-test-suite-driver.md" ]] && \
   grep -q "members:" ".chant/specs/002-test-suite-driver.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Driver spec missing or invalid"
    ((FAILED++))
fi

# Test 3: Verify all three member specs exist
echo -n "✓ Checking member specs exist... "
if [[ -f ".chant/specs/002-test-suite-driver.1.md" ]] && \
   [[ -f ".chant/specs/002-test-suite-driver.2.md" ]] && \
   [[ -f ".chant/specs/002-test-suite-driver.3.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing member specs"
    ((FAILED++))
fi

# Test 4: Verify test pattern context files exist
echo -n "✓ Checking TDD context files exist... "
if [[ -f ".chant/context/tdd-standards/coverage-requirements.md" ]] && \
   [[ -f ".chant/context/tdd-standards/test-patterns.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing TDD context files"
    ((FAILED++))
fi

# Test 5: Verify member specs contain test case counts
echo -n "✓ Checking member specs have test case counts... "
if grep -qE "(16 test cases|test cases: 16)" ".chant/specs/002-test-suite-driver.1.md" && \
   grep -qE "(4 test cases|test cases: 4)" ".chant/specs/002-test-suite-driver.2.md" && \
   grep -qE "(4 test cases|test cases: 4)" ".chant/specs/002-test-suite-driver.3.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Member specs missing test case counts"
    ((FAILED++))
fi

# Test 6: Verify src and tests directories exist
echo -n "✓ Checking project structure exists... "
if [[ -d "src" ]] && [[ -d "tests" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing src or tests directories"
    ((FAILED++))
fi

# Test 7: Verify coverage analysis spec has target_files
echo -n "✓ Checking research spec has target_files... "
if grep -q "target_files:" ".chant/specs/001-coverage-analysis.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Coverage analysis spec missing target_files"
    ((FAILED++))
fi

# Test 8: Verify README links are valid
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
