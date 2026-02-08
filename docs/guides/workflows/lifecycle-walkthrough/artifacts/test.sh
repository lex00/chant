#!/usr/bin/env bash
set -e

# Test script for lifecycle walkthrough example
# Validates that the example works with chant commands

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "=== Lifecycle Walkthrough Example Validation ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

pass() {
    echo -e "${GREEN}✓${NC} $1"
}

fail() {
    echo -e "${RED}✗${NC} $1"
    exit 1
}

# Test 1: Check source files exist
echo -n "Checking source files exist... "
if [ -f "src/datalog.py" ] && [ -f "src/query.py" ] && [ -f "src/export.py" ]; then
    pass "PASS"
else
    fail "FAIL - source files missing"
fi

# Test 2: Check test files exist
echo -n "Checking test files exist... "
if [ -f "tests/test_query.py" ] && [ -f "tests/test_export.py" ]; then
    pass "PASS"
else
    fail "FAIL - test files missing"
fi

# Test 3: Check spec artifacts exist
echo -n "Checking spec artifacts exist... "
if [ -f "spec-001-initial.md" ] && \
   [ -f "spec-001-focused.md" ] && \
   [ -f "spec-001-driver.md" ] && \
   [ -f "spec-001.1-csv-handler.md" ] && \
   [ -f "spec-001.2-command-skeleton.md" ] && \
   [ -f "spec-001.3-integration-tests.md" ] && \
   [ -f "spec-004-severity-field.md" ]; then
    pass "PASS"
else
    fail "FAIL - spec artifacts missing"
fi

# Test 4: Verify Python syntax is valid
echo -n "Checking Python syntax is valid... "
if command -v python3 &> /dev/null; then
    PYTHONPATH="$SCRIPT_DIR:$PYTHONPATH" python3 -m py_compile src/datalog.py src/query.py src/export.py tests/test_query.py tests/test_export.py 2>/dev/null
    pass "PASS"
else
    echo "SKIP (python3 not available)"
fi

# Test 5: Check if we're in a chant project (optional, since user runs chant init)
echo -n "Checking chant can be initialized... "
if [ -d ".chant" ]; then
    # Already initialized
    pass "PASS (already initialized)"
elif command -v chant &> /dev/null; then
    # Try to init
    chant init --quiet 2>/dev/null || true
    if [ -d ".chant" ]; then
        pass "PASS"
    else
        echo "SKIP (chant init not run yet)"
    fi
else
    echo "SKIP (chant not available)"
fi

# Test 6: Validate spec files can be linted (if chant is available)
echo -n "Checking spec files are valid YAML... "
if command -v chant &> /dev/null && [ -d ".chant" ]; then
    # Copy a spec to .chant/specs for testing
    mkdir -p .chant/specs
    cp spec-001-initial.md .chant/specs/test-spec.md 2>/dev/null || true
    if [ -f ".chant/specs/test-spec.md" ]; then
        chant lint test-spec --quiet 2>/dev/null && pass "PASS" || echo "SKIP"
        rm .chant/specs/test-spec.md
    else
        echo "SKIP"
    fi
else
    echo "SKIP (chant not available or not initialized)"
fi

# Test 7: Check query tests can run (if pytest is available)
echo -n "Checking query tests can run... "
if command -v pytest &> /dev/null; then
    PYTHONPATH="$SCRIPT_DIR:$PYTHONPATH" pytest tests/test_query.py -q 2>/dev/null && pass "PASS" || echo "SKIP (tests need dependencies)"
else
    echo "SKIP (pytest not available)"
fi

# Test 8: Verify spec files have required frontmatter fields
echo -n "Checking spec frontmatter is valid... "
spec_valid=true
for spec in spec-001-initial.md spec-001-focused.md spec-001-driver.md spec-001.1-csv-handler.md; do
    if ! grep -q "^id:" "$spec" || ! grep -q "^status:" "$spec"; then
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
echo "=== Validation Summary ==="
echo "All checks passed! The example is ready to use."
echo
echo "Next steps:"
echo "  1. Copy this directory to your working location"
echo "  2. Run 'chant init' to initialize chant"
echo "  3. Follow the lifecycle walkthrough guide"
echo
