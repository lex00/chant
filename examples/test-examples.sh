#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track execution results
declare -a FAILED_SPECS=()
declare -a EXPECTED_FAILURES=()
declare -a UNEXPECTED_FAILURES=()

# Available examples
EXAMPLES=(
  "approval-workflow"
  "kpi-okr-workflow"
  "oss-maintainer-workflow"
  "research-workflow"
  "tdd-workflow"
)

usage() {
  cat <<EOF
Usage: $0 [COMMAND] [EXAMPLE_NAME]

Commands:
  all                Run all examples sequentially
  <example-name>     Run a specific example
  --help, -h         Show this help message

Available examples:
  - approval-workflow          (3 independent specs)
  - kpi-okr-workflow          (1 driver spec with members)
  - oss-maintainer-workflow   (6 specs in chain)
  - research-workflow         (2 independent sub-examples)
  - tdd-workflow              (1 driver spec with members)

Example usage:
  # Run all examples
  $0 all

  # Run a specific example
  $0 kpi-okr-workflow

  # Show help
  $0 --help

Execution Patterns:
  - Driver pattern: Single driver spec that coordinates member specs
    Example: kpi-okr-workflow, tdd-workflow
    Command: chant work <driver-spec-id>

  - Chain pattern: Multiple specs with dependencies
    Example: oss-maintainer-workflow
    Command: chant work --chain <spec-ids...>

  - Independent pattern: Multiple independent specs
    Example: approval-workflow, research-workflow
    Command: Run each spec separately

EOF
}

check_claude_cli() {
  if ! command -v claude &> /dev/null; then
    echo -e "${RED}Error: Claude Code CLI not found${NC}"
    echo "Please install Claude Code CLI to run examples"
    echo "Visit: https://github.com/anthropics/claude-code"
    exit 1
  fi
}

has_subpaths() {
  local example_dir="$1"

  # Check if there are subdirectories with .chant directories
  local subpaths=$(find "$example_dir" -mindepth 2 -maxdepth 2 -type d -name ".chant" 2>/dev/null)

  if [ -n "$subpaths" ]; then
    echo "true"
  else
    echo "false"
  fi
}

get_subpaths() {
  local example_dir="$1"

  # Find all subdirectories containing .chant
  local subpaths=$(find "$example_dir" -mindepth 2 -maxdepth 2 -type d -name ".chant" -exec dirname {} \; 2>/dev/null | sort)

  echo "$subpaths"
}

validate_example() {
  local example_name="$1"
  local example_dir="$EXAMPLES_DIR/$example_name"

  if [ ! -d "$example_dir" ]; then
    echo -e "${RED}Error: Example directory not found: $example_dir${NC}"
    exit 1
  fi

  # Check for multi-path structure
  if [ "$(has_subpaths "$example_dir")" == "true" ]; then
    return 0
  fi

  if [ ! -d "$example_dir/.chant/specs" ]; then
    echo -e "${RED}Error: No .chant/specs directory in $example_name${NC}"
    exit 1
  fi
}

detect_pattern() {
  local example_dir="$1"
  local specs_dir="$example_dir/.chant/specs"

  # Check for driver specs
  local driver_count=$(grep -l "^type: driver" "$specs_dir"/*.md 2>/dev/null | wc -l | tr -d ' ')

  # Check for chain dependencies
  local chain_count=$(grep -l "^depends_on:" "$specs_dir"/*.md 2>/dev/null | wc -l | tr -d ' ')

  if [ "$driver_count" -gt 0 ]; then
    echo "driver"
  elif [ "$chain_count" -gt 1 ]; then
    echo "chain"
  else
    echo "independent"
  fi
}

get_driver_spec_id() {
  local example_dir="$1"
  local specs_dir="$example_dir/.chant/specs"

  # Find the driver spec file
  local driver_spec=$(grep -l "^type: driver" "$specs_dir"/*.md 2>/dev/null | head -n 1)

  if [ -z "$driver_spec" ]; then
    echo ""
    return
  fi

  # Extract spec ID from filename (e.g., 002-driver-churn-fixes.md -> 002)
  basename "$driver_spec" .md
}

get_chain_spec_ids() {
  local example_dir="$1"
  local specs_dir="$example_dir/.chant/specs"

  # Get all spec files that have dependencies, sorted by dependency order
  # For simplicity, just list all specs in order
  local spec_files=$(ls "$specs_dir"/*.md 2>/dev/null | sort)

  local spec_ids=()
  for spec_file in $spec_files; do
    local spec_id=$(basename "$spec_file" .md)
    # Skip member specs (containing dots like .1, .2, etc)
    if [[ ! "$spec_id" =~ \.[0-9]+$ ]]; then
      spec_ids+=("$spec_id")
    fi
  done

  echo "${spec_ids[@]}"
}

get_independent_spec_ids() {
  local example_dir="$1"
  local specs_dir="$example_dir/.chant/specs"

  # Get all spec files
  local spec_files=$(ls "$specs_dir"/*.md 2>/dev/null | sort)

  local spec_ids=()
  for spec_file in $spec_files; do
    local spec_id=$(basename "$spec_file" .md)
    # Skip member specs and auto-generated specs
    if [[ ! "$spec_id" =~ \.[0-9]+$ ]]; then
      spec_ids+=("$spec_id")
    fi
  done

  echo "${spec_ids[@]}"
}

run_chant_work() {
  local spec_id="$1"
  local context="$2"

  set +e
  local output
  output=$(chant work "$spec_id" 2>&1)
  local exit_code=$?
  set -e

  echo "$output"

  if [ $exit_code -ne 0 ]; then
    # Check for expected failure patterns
    if echo "$output" | grep -q "requires approval\|status: rejected\|approval required\|has been rejected"; then
      EXPECTED_FAILURES+=("$context/$spec_id (requires approval or rejected)")
      return 2
    elif echo "$output" | grep -q "config.md not found\|No such file"; then
      UNEXPECTED_FAILURES+=("$context/$spec_id (missing config file)")
      return 1
    else
      UNEXPECTED_FAILURES+=("$context/$spec_id (exit code: $exit_code)")
      return 1
    fi
  fi

  return 0
}

run_chant_work_chain() {
  local context="$1"
  shift
  local chain_specs=("$@")

  set +e
  local output
  output=$(chant work --chain "${chain_specs[@]}" 2>&1)
  local exit_code=$?
  set -e

  echo "$output"

  if [ $exit_code -ne 0 ]; then
    # Check for expected failure patterns
    if echo "$output" | grep -q "requires approval\|status: rejected\|approval required\|has been rejected"; then
      EXPECTED_FAILURES+=("$context/chain:${chain_specs[*]} (requires approval or rejected)")
      return 2
    elif echo "$output" | grep -q "config.md not found\|No such file"; then
      UNEXPECTED_FAILURES+=("$context/chain:${chain_specs[*]} (missing config file)")
      return 1
    else
      UNEXPECTED_FAILURES+=("$context/chain:${chain_specs[*]} (exit code: $exit_code)")
      return 1
    fi
  fi

  return 0
}

run_subpath() {
  local subpath_dir="$1"
  local subpath_name=$(basename "$subpath_dir")

  echo -e "${YELLOW}  Running sub-path: $subpath_name${NC}"

  local pattern=$(detect_pattern "$subpath_dir")
  echo -e "${YELLOW}  Detected pattern: $pattern${NC}"

  cd "$subpath_dir"

  local had_failures=0

  case "$pattern" in
    driver)
      local driver_spec=$(get_driver_spec_id "$subpath_dir")
      if [ -z "$driver_spec" ]; then
        echo -e "${RED}Error: No driver spec found${NC}"
        UNEXPECTED_FAILURES+=("$subpath_name (no driver spec found)")
        return 1
      fi
      echo "  Running: chant work $driver_spec"
      if ! run_chant_work "$driver_spec" "$subpath_name"; then
        local result=$?
        [ $result -eq 1 ] && had_failures=1
      fi
      ;;

    chain)
      local chain_specs=($(get_chain_spec_ids "$subpath_dir"))
      if [ ${#chain_specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No chain specs found${NC}"
        UNEXPECTED_FAILURES+=("$subpath_name (no chain specs found)")
        return 1
      fi
      echo "  Running: chant work --chain ${chain_specs[*]}"
      if ! run_chant_work_chain "$subpath_name" "${chain_specs[@]}"; then
        local result=$?
        [ $result -eq 1 ] && had_failures=1
      fi
      ;;

    independent)
      local specs=($(get_independent_spec_ids "$subpath_dir"))
      if [ ${#specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No specs found${NC}"
        UNEXPECTED_FAILURES+=("$subpath_name (no specs found)")
        return 1
      fi
      echo "  Running ${#specs[@]} independent specs"
      for spec in "${specs[@]}"; do
        echo "    Running: chant work $spec"
        if ! run_chant_work "$spec" "$subpath_name"; then
          local result=$?
          [ $result -eq 1 ] && had_failures=1
        fi
      done
      ;;
  esac

  # Run assertions if test script exists
  if [ -x "$subpath_dir/test-assertions.sh" ]; then
    echo "  Running assertions for $subpath_name"
    set +e
    "$subpath_dir/test-assertions.sh"
    local assertion_result=$?
    set -e
    if [ $assertion_result -ne 0 ]; then
      UNEXPECTED_FAILURES+=("$subpath_name/test-assertions.sh (exit code: $assertion_result)")
      had_failures=1
    fi
  fi

  return $had_failures
}

run_example() {
  local example_name="$1"
  local example_dir="$EXAMPLES_DIR/$example_name"

  echo -e "${GREEN}Running example: $example_name${NC}"

  validate_example "$example_name"

  local had_failures=0

  # Check if this is a multi-path example
  if [ "$(has_subpaths "$example_dir")" == "true" ]; then
    echo -e "${YELLOW}Detected multi-path structure${NC}"
    local subpaths=$(get_subpaths "$example_dir")

    for subpath in $subpaths; do
      if ! run_subpath "$subpath"; then
        had_failures=1
      fi
    done

    cd "$SCRIPT_DIR"
    if [ $had_failures -eq 0 ]; then
      echo -e "${GREEN}✓ Completed: $example_name${NC}"
    else
      echo -e "${YELLOW}⚠ Completed with failures: $example_name${NC}"
    fi
    return $had_failures
  fi

  local pattern=$(detect_pattern "$example_dir")
  echo -e "${YELLOW}Detected pattern: $pattern${NC}"

  cd "$example_dir"

  case "$pattern" in
    driver)
      local driver_spec=$(get_driver_spec_id "$example_dir")
      if [ -z "$driver_spec" ]; then
        echo -e "${RED}Error: No driver spec found${NC}"
        UNEXPECTED_FAILURES+=("$example_name (no driver spec found)")
        cd "$SCRIPT_DIR"
        return 1
      fi
      echo "Running: chant work $driver_spec"
      if ! run_chant_work "$driver_spec" "$example_name"; then
        local result=$?
        [ $result -eq 1 ] && had_failures=1
      fi
      ;;

    chain)
      local chain_specs=($(get_chain_spec_ids "$example_dir"))
      if [ ${#chain_specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No chain specs found${NC}"
        UNEXPECTED_FAILURES+=("$example_name (no chain specs found)")
        cd "$SCRIPT_DIR"
        return 1
      fi
      echo "Running: chant work --chain ${chain_specs[*]}"
      if ! run_chant_work_chain "$example_name" "${chain_specs[@]}"; then
        local result=$?
        [ $result -eq 1 ] && had_failures=1
      fi
      ;;

    independent)
      local specs=($(get_independent_spec_ids "$example_dir"))
      if [ ${#specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No specs found${NC}"
        UNEXPECTED_FAILURES+=("$example_name (no specs found)")
        cd "$SCRIPT_DIR"
        return 1
      fi
      echo "Running ${#specs[@]} independent specs"
      for spec in "${specs[@]}"; do
        echo "  Running: chant work $spec"
        if ! run_chant_work "$spec" "$example_name"; then
          local result=$?
          [ $result -eq 1 ] && had_failures=1
        fi
      done
      ;;
  esac

  cd "$SCRIPT_DIR"
  if [ $had_failures -eq 0 ]; then
    echo -e "${GREEN}✓ Completed: $example_name${NC}"
  else
    echo -e "${YELLOW}⚠ Completed with failures: $example_name${NC}"
  fi
  return $had_failures
}

print_summary() {
  echo ""
  echo "========================================"
  echo "EXECUTION SUMMARY"
  echo "========================================"

  if [ ${#EXPECTED_FAILURES[@]} -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}Expected failures (approval-required/rejected):${NC}"
    for failure in "${EXPECTED_FAILURES[@]}"; do
      echo "  ℹ $failure"
    done
  fi

  if [ ${#UNEXPECTED_FAILURES[@]} -gt 0 ]; then
    echo ""
    echo -e "${RED}Unexpected failures:${NC}"
    for failure in "${UNEXPECTED_FAILURES[@]}"; do
      echo "  ✗ $failure"
    done
    echo ""
    echo -e "${RED}Test suite FAILED${NC}"
    return 1
  else
    echo ""
    if [ ${#EXPECTED_FAILURES[@]} -gt 0 ]; then
      echo -e "${GREEN}All examples completed (with expected approval/rejection notices)${NC}"
    else
      echo -e "${GREEN}All examples completed successfully!${NC}"
    fi
    return 0
  fi
}

run_all_examples() {
  echo -e "${GREEN}Running all examples${NC}"

  for example in "${EXAMPLES[@]}"; do
    run_example "$example" || true
    echo ""
  done

  print_summary
}

main() {
  if [ $# -eq 0 ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    usage
    exit 0
  fi

  check_claude_cli

  case "$1" in
    all)
      if run_all_examples; then
        exit 0
      else
        exit 1
      fi
      ;;
    *)
      # Check if the argument is a valid example
      local is_valid=0
      for example in "${EXAMPLES[@]}"; do
        if [ "$1" == "$example" ]; then
          is_valid=1
          break
        fi
      done

      if [ $is_valid -eq 0 ]; then
        echo -e "${RED}Error: Unknown example '$1'${NC}"
        echo ""
        usage
        exit 1
      fi

      if run_example "$1"; then
        print_summary
        exit 0
      else
        print_summary
        exit 1
      fi
      ;;
  esac
}

main "$@"
