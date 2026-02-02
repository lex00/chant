#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== OSS Maintainer Workflow Validation ==="

# Test 1: Verify all 6 phase specs exist
echo -n "✓ Checking all 6 phase specs exist... "
if [[ -f ".chant/specs/001-comprehension.md" ]] && \
   [[ -f ".chant/specs/002-reproduction.md" ]] && \
   [[ -f ".chant/specs/003-root-cause.md" ]] && \
   [[ -f ".chant/specs/004-sprawl.md" ]] && \
   [[ -f ".chant/specs/005-fork-fix.md" ]] && \
   [[ -f ".chant/specs/006-upstream-pr.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing phase specs"
    ((FAILED++))
fi

# Test 2: Verify comprehension spec is type: research
echo -n "✓ Checking phase 1 is research spec... "
if grep -q "type: research" ".chant/specs/001-comprehension.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Phase 1 spec missing type: research"
    ((FAILED++))
fi

# Test 3: Verify root cause spec references comprehension output
echo -n "✓ Checking phase 3 references phase 1... "
if grep -q "informed_by:" ".chant/specs/003-root-cause.md" && \
   (grep -q "issue-42-comprehension" ".chant/specs/003-root-cause.md" || grep -q "001" ".chant/specs/003-root-cause.md"); then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Phase 3 missing informed_by reference"
    ((FAILED++))
fi

# Test 4: Verify sprawl spec is type: research
echo -n "✓ Checking phase 4 is research spec... "
if grep -q "type: research" ".chant/specs/004-sprawl.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Phase 4 spec missing type: research"
    ((FAILED++))
fi

# Test 5: Verify fork-fix spec has depends_on
echo -n "✓ Checking phase 5 has dependencies... "
if grep -q "depends_on:" ".chant/specs/005-fork-fix.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Phase 5 spec missing depends_on field"
    ((FAILED++))
fi

# Test 6: Verify upstream-pr spec documents human gate
echo -n "✓ Checking phase 6 has human gate documentation... "
if grep -qE "(human|review|checklist|gate)" ".chant/specs/006-upstream-pr.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Phase 6 spec missing human gate documentation"
    ((FAILED++))
fi

# Test 7: Verify research specs have target_files
echo -n "✓ Checking research specs have target_files... "
if grep -q "target_files:" ".chant/specs/001-comprehension.md" && \
   grep -q "target_files:" ".chant/specs/003-root-cause.md" && \
   grep -q "target_files:" ".chant/specs/004-sprawl.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research specs missing target_files"
    ((FAILED++))
fi

# Test 8: Verify src and tests directories exist
echo -n "✓ Checking project structure exists... "
if [[ -d "src" ]] && [[ -d "tests" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing src or tests directories"
    ((FAILED++))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
