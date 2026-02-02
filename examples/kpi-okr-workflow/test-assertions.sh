#!/usr/bin/env bash
set -e

EXAMPLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$EXAMPLE_DIR"

FAILED=0
PASSED=0

echo "=== KPI/OKR Workflow Validation ==="

# Test 1: Verify research spec exists with informed_by
echo -n "✓ Checking research spec exists... "
if [[ -f ".chant/specs/001-research-churn-drivers.md" ]] && \
   grep -q "informed_by:" ".chant/specs/001-research-churn-drivers.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing or invalid research spec"
    ((FAILED++))
fi

# Test 2: Verify driver spec exists with members field
echo -n "✓ Checking driver spec has members... "
if [[ -f ".chant/specs/002-driver-churn-fixes.md" ]] && \
   grep -q "members:" ".chant/specs/002-driver-churn-fixes.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Driver spec missing or invalid"
    ((FAILED++))
fi

# Test 3: Verify all three member specs exist
echo -n "✓ Checking member specs exist... "
if [[ -f ".chant/specs/002-driver-churn-fixes.1.md" ]] && \
   [[ -f ".chant/specs/002-driver-churn-fixes.2.md" ]] && \
   [[ -f ".chant/specs/002-driver-churn-fixes.3.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing member specs"
    ((FAILED++))
fi

# Test 4: Verify context files exist
echo -n "✓ Checking context files exist... "
if [[ -f ".chant/context/kpi-churn-q1/datadog-churn-metrics.md" ]] && \
   [[ -f ".chant/context/kpi-churn-q1/zendesk-support-patterns.md" ]] && \
   [[ -f ".chant/context/kpi-churn-q1/user-survey-summary.md" ]]; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Missing context files"
    ((FAILED++))
fi

# Test 5: Verify member specs contain churn impact metrics
echo -n "✓ Checking member specs have impact metrics... "
if grep -q "3.5pp" ".chant/specs/002-driver-churn-fixes.1.md" && \
   grep -q "1.5pp" ".chant/specs/002-driver-churn-fixes.2.md" && \
   grep -q "1.2pp" ".chant/specs/002-driver-churn-fixes.3.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Member specs missing impact metrics"
    ((FAILED++))
fi

# Test 6: Verify research spec has type: research
echo -n "✓ Checking research spec type... "
if grep -q "type: research" ".chant/specs/001-research-churn-drivers.md"; then
    echo "PASS"
    ((PASSED++))
else
    echo "FAIL - Research spec missing type field"
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
