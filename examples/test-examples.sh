#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

validate_example() {
  local example_name="$1"
  local example_dir="$EXAMPLES_DIR/$example_name"

  if [ ! -d "$example_dir" ]; then
    echo -e "${RED}Error: Example directory not found: $example_dir${NC}"
    exit 1
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

run_example() {
  local example_name="$1"
  local example_dir="$EXAMPLES_DIR/$example_name"

  echo -e "${GREEN}Running example: $example_name${NC}"

  validate_example "$example_name"

  local pattern=$(detect_pattern "$example_dir")
  echo -e "${YELLOW}Detected pattern: $pattern${NC}"

  cd "$example_dir"

  case "$pattern" in
    driver)
      local driver_spec=$(get_driver_spec_id "$example_dir")
      if [ -z "$driver_spec" ]; then
        echo -e "${RED}Error: No driver spec found${NC}"
        return 1
      fi
      echo "Running: chant work $driver_spec"
      chant work "$driver_spec"
      ;;

    chain)
      local chain_specs=($(get_chain_spec_ids "$example_dir"))
      if [ ${#chain_specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No chain specs found${NC}"
        return 1
      fi
      echo "Running: chant work --chain ${chain_specs[*]}"
      chant work --chain "${chain_specs[@]}"
      ;;

    independent)
      local specs=($(get_independent_spec_ids "$example_dir"))
      if [ ${#specs[@]} -eq 0 ]; then
        echo -e "${RED}Error: No specs found${NC}"
        return 1
      fi
      echo "Running ${#specs[@]} independent specs"
      for spec in "${specs[@]}"; do
        echo "  Running: chant work $spec"
        chant work "$spec"
      done
      ;;
  esac

  cd "$SCRIPT_DIR"
  echo -e "${GREEN}✓ Completed: $example_name${NC}"
  return 0
}

run_all_examples() {
  echo -e "${GREEN}Running all examples${NC}"
  local failed_examples=()

  for example in "${EXAMPLES[@]}"; do
    if ! run_example "$example"; then
      failed_examples+=("$example")
    fi
    echo ""
  done

  if [ ${#failed_examples[@]} -gt 0 ]; then
    echo -e "${RED}Failed examples:${NC}"
    for example in "${failed_examples[@]}"; do
      echo "  - $example"
    done
    exit 1
  else
    echo -e "${GREEN}All examples completed successfully!${NC}"
  fi
}

main() {
  if [ $# -eq 0 ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    usage
    exit 0
  fi

  check_claude_cli

  case "$1" in
    all)
      run_all_examples
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

      run_example "$1"
      ;;
  esac
}

main "$@"
