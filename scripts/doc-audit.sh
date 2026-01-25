#!/usr/bin/env bash
#
# doc-audit.sh - Documentation freshness tracking for chant
#
# Usage:
#   ./scripts/doc-audit.sh status          Show audit status for all modules
#   ./scripts/doc-audit.sh mark <path>     Update audit timestamp for a module
#   ./scripts/doc-audit.sh stale           Show only stale modules
#   ./scripts/doc-audit.sh orphans         Check for orphaned mappings
#
# Staleness is determined by comparing:
# - Git modification date of source file
# - last_audit timestamp in doc-audit-map.toml
#
# A module is stale if:
# - Source changed after last_audit, OR
# - No last_audit exists (never audited), OR
# - last_audit is older than 90 days (threshold-based staleness)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AUDIT_MAP="$PROJECT_ROOT/docs/doc-audit-map.toml"
STALE_THRESHOLD_DAYS=90

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse the audit map file and extract mappings
parse_mappings() {
    local current_file=""
    local in_mapping=false

    while IFS= read -r line; do
        # Match [mappings."src/file.rs"] pattern
        if [[ "$line" =~ ^\[mappings\.\"(.+)\"\] ]]; then
            current_file="${BASH_REMATCH[1]}"
            in_mapping=true
            echo "FILE:$current_file"
        elif [[ "$in_mapping" == true && "$line" =~ ^last_audit\ *=\ *\"([0-9]{4}-[0-9]{2}-[0-9]{2})\" ]]; then
            echo "AUDIT:${BASH_REMATCH[1]}"
        elif [[ "$in_mapping" == true && "$line" =~ ^docs\ *=\ *\[(.+)\] ]]; then
            echo "DOCS:${BASH_REMATCH[1]}"
        elif [[ "$in_mapping" == true && "$line" =~ ^ignore\ *=\ *true ]]; then
            echo "IGNORE:true"
        elif [[ "$line" =~ ^\[ && ! "$line" =~ ^\[mappings ]]; then
            in_mapping=false
        fi
    done < "$AUDIT_MAP"
}

# Get git modification date for a file (YYYY-MM-DD format)
get_git_mod_date() {
    local file="$1"
    cd "$PROJECT_ROOT"
    git log -1 --format="%cs" -- "$file" 2>/dev/null || echo ""
}

# Compare dates, returns 0 if date1 > date2
date_after() {
    local date1="$1"
    local date2="$2"
    [[ "$date1" > "$date2" ]]
}

# Check if date is older than threshold days
date_older_than_threshold() {
    local audit_date="$1"
    local threshold_date

    # Calculate threshold date (today - STALE_THRESHOLD_DAYS)
    if [[ "$(uname)" == "Darwin" ]]; then
        threshold_date=$(date -v-${STALE_THRESHOLD_DAYS}d +%Y-%m-%d)
    else
        threshold_date=$(date -d "$STALE_THRESHOLD_DAYS days ago" +%Y-%m-%d)
    fi

    [[ "$audit_date" < "$threshold_date" ]]
}

# Show status for all modules
cmd_status() {
    local show_all=${1:-true}
    local current_file=""
    local current_audit=""
    local current_docs=""
    local current_ignore=false
    local total=0
    local stale=0
    local fresh=0
    local ignored=0
    local orphaned=0

    echo -e "${BLUE}Documentation Audit Status${NC}"
    echo "=============================="
    echo ""

    while IFS= read -r line; do
        case "$line" in
            FILE:*)
                # Process previous file if exists
                if [[ -n "$current_file" ]]; then
                    process_file "$current_file" "$current_audit" "$current_docs" "$current_ignore" "$show_all"
                fi
                current_file="${line#FILE:}"
                current_audit=""
                current_docs=""
                current_ignore=false
                ;;
            AUDIT:*)
                current_audit="${line#AUDIT:}"
                ;;
            DOCS:*)
                current_docs="${line#DOCS:}"
                ;;
            IGNORE:*)
                current_ignore=true
                ;;
        esac
    done < <(parse_mappings)

    # Process last file
    if [[ -n "$current_file" ]]; then
        process_file "$current_file" "$current_audit" "$current_docs" "$current_ignore" "$show_all"
    fi

    echo ""
    echo "=============================="
    echo -e "Total: $total | ${GREEN}Fresh: $fresh${NC} | ${YELLOW}Stale: $stale${NC} | Ignored: $ignored"

    if [[ $orphaned -gt 0 ]]; then
        echo -e "${RED}Orphaned: $orphaned${NC}"
    fi
}

process_file() {
    local file="$1"
    local audit_date="$2"
    local docs="$3"
    local ignore="$4"
    local show_all="$5"

    ((total++)) || true

    # Check if file exists
    if [[ ! -f "$PROJECT_ROOT/$file" ]]; then
        echo -e "${RED}ORPHAN${NC}  $file"
        echo "        File not found - mapping may be stale"
        ((orphaned++)) || true
        return
    fi

    # Skip ignored files
    if [[ "$ignore" == true ]]; then
        ((ignored++)) || true
        if [[ "$show_all" == true ]]; then
            echo -e "SKIP    $file (ignored)"
        fi
        return
    fi

    local git_date
    git_date=$(get_git_mod_date "$file")

    local status=""
    local reason=""

    if [[ -z "$audit_date" ]]; then
        status="STALE"
        reason="never audited"
        ((stale++)) || true
    elif [[ -n "$git_date" ]] && date_after "$git_date" "$audit_date"; then
        status="STALE"
        reason="modified $git_date, audited $audit_date"
        ((stale++)) || true
    elif date_older_than_threshold "$audit_date"; then
        status="STALE"
        reason="audit older than ${STALE_THRESHOLD_DAYS} days ($audit_date)"
        ((stale++)) || true
    else
        status="FRESH"
        reason="audited $audit_date"
        ((fresh++)) || true
    fi

    if [[ "$status" == "STALE" ]]; then
        echo -e "${YELLOW}STALE${NC}   $file"
        echo "        $reason"
    elif [[ "$show_all" == true ]]; then
        echo -e "${GREEN}FRESH${NC}   $file"
        echo "        $reason"
    fi
}

# Show only stale modules
cmd_stale() {
    # Reuse status logic but only show stale
    local current_file=""
    local current_audit=""
    local current_docs=""
    local current_ignore=false

    echo -e "${BLUE}Stale Modules${NC}"
    echo "=============="
    echo ""

    while IFS= read -r line; do
        case "$line" in
            FILE:*)
                if [[ -n "$current_file" ]]; then
                    show_if_stale "$current_file" "$current_audit" "$current_ignore"
                fi
                current_file="${line#FILE:}"
                current_audit=""
                current_ignore=false
                ;;
            AUDIT:*)
                current_audit="${line#AUDIT:}"
                ;;
            IGNORE:*)
                current_ignore=true
                ;;
        esac
    done < <(parse_mappings)

    if [[ -n "$current_file" ]]; then
        show_if_stale "$current_file" "$current_audit" "$current_ignore"
    fi
}

show_if_stale() {
    local file="$1"
    local audit_date="$2"
    local ignore="$3"

    [[ "$ignore" == true ]] && return
    [[ ! -f "$PROJECT_ROOT/$file" ]] && return

    local git_date
    git_date=$(get_git_mod_date "$file")

    if [[ -z "$audit_date" ]]; then
        echo "$file (never audited)"
    elif [[ -n "$git_date" ]] && date_after "$git_date" "$audit_date"; then
        echo "$file (modified: $git_date, audited: $audit_date)"
    elif date_older_than_threshold "$audit_date"; then
        echo "$file (audit too old: $audit_date)"
    fi
}

# Check for orphaned mappings
cmd_orphans() {
    echo -e "${BLUE}Checking for Orphaned Mappings${NC}"
    echo "================================"
    echo ""

    local found_orphan=false

    while IFS= read -r line; do
        if [[ "$line" =~ ^FILE:(.+) ]]; then
            local file="${BASH_REMATCH[1]}"
            if [[ ! -f "$PROJECT_ROOT/$file" ]]; then
                echo -e "${RED}ORPHAN${NC}: $file"
                echo "  HINT: File may have been renamed. Update doc-audit-map.toml"
                found_orphan=true
            fi
        fi
    done < <(parse_mappings)

    if [[ "$found_orphan" == false ]]; then
        echo -e "${GREEN}No orphaned mappings found.${NC}"
    fi
}

# Mark a module as audited (updates both tracking file and docstring)
cmd_mark() {
    local file="$1"
    local today
    today=$(date +%Y-%m-%d)

    if [[ ! -f "$PROJECT_ROOT/$file" ]]; then
        echo -e "${RED}Error: File not found: $file${NC}"
        exit 1
    fi

    echo "Marking $file as audited on $today"

    # Update tracking file using Python for reliable cross-platform editing
    if grep -q "^\[mappings\.\"$file\"\]" "$AUDIT_MAP" 2>/dev/null || \
       grep -q "^\[mappings\\.\"$(echo "$file" | sed 's|/|\\/|g')\"\]" "$AUDIT_MAP" 2>/dev/null; then
        python3 - "$AUDIT_MAP" "$file" "$today" << 'PYTHON_SCRIPT'
import sys
import re

audit_map_path = sys.argv[1]
file_path = sys.argv[2]
today = sys.argv[3]

with open(audit_map_path, 'r') as f:
    content = f.read()

# Escape special regex characters in file path
escaped_path = re.escape(file_path)

# Pattern to match the mapping section
section_pattern = rf'\[mappings\."{escaped_path}"\]'

# Check if section exists
if not re.search(section_pattern, content):
    print(f"Warning: No mapping found for {file_path}")
    sys.exit(0)

# Find the section and update or add last_audit
lines = content.split('\n')
in_section = False
found_last_audit = False
new_lines = []

for i, line in enumerate(lines):
    if re.match(section_pattern, line):
        in_section = True
        new_lines.append(line)
        continue

    if in_section:
        # Check if we hit a new section
        if line.startswith('[') and not line.startswith('[mappings."' + file_path):
            # If we haven't found last_audit, add it before this new section
            if not found_last_audit:
                new_lines.append(f'last_audit = "{today}"')
            in_section = False
            new_lines.append(line)
            continue

        # Update existing last_audit (commented or not)
        if 'last_audit' in line:
            new_lines.append(f'last_audit = "{today}"')
            found_last_audit = True
            continue

    new_lines.append(line)

# Handle case where section is at end of file
if in_section and not found_last_audit:
    # Find where to insert (after docs = line)
    for i in range(len(new_lines) - 1, -1, -1):
        if new_lines[i].startswith('docs = '):
            new_lines.insert(i + 1, f'last_audit = "{today}"')
            break

with open(audit_map_path, 'w') as f:
    f.write('\n'.join(new_lines))

print("Updated tracking file")
PYTHON_SCRIPT
        echo -e "${GREEN}Updated tracking file${NC}"
    else
        echo -e "${YELLOW}Warning: No mapping found for $file in tracking file${NC}"
    fi

    # Update docstring marker if it exists
    if grep -q "# Doc Audit" "$PROJECT_ROOT/$file"; then
        # Update audited date in docstring - handle both (pending) and date formats
        if grep -q "audited: (pending)" "$PROJECT_ROOT/$file"; then
            sed -i.bak "s|//! - audited: (pending)|//! - audited: $today|" "$PROJECT_ROOT/$file"
        else
            sed -i.bak "s|//! - audited: [0-9-]*|//! - audited: $today|" "$PROJECT_ROOT/$file"
        fi
        rm -f "$PROJECT_ROOT/$file.bak"
        echo -e "${GREEN}Updated docstring marker${NC}"
    else
        echo -e "${YELLOW}Note: No docstring marker found in $file${NC}"
    fi

    echo -e "${GREEN}Done!${NC}"
}

# Show usage
usage() {
    echo "Documentation Audit Tool"
    echo ""
    echo "Usage:"
    echo "  $0 status         Show audit status for all modules"
    echo "  $0 stale          Show only stale modules"
    echo "  $0 orphans        Check for orphaned mappings"
    echo "  $0 mark <path>    Mark a module as audited"
    echo ""
    echo "Examples:"
    echo "  $0 status"
    echo "  $0 mark src/spec.rs"
}

# Main entry point
main() {
    if [[ ! -f "$AUDIT_MAP" ]]; then
        echo -e "${RED}Error: Audit map not found at $AUDIT_MAP${NC}"
        exit 1
    fi

    local cmd="${1:-status}"

    case "$cmd" in
        status)
            cmd_status true
            ;;
        stale)
            cmd_stale
            ;;
        orphans)
            cmd_orphans
            ;;
        mark)
            if [[ -z "${2:-}" ]]; then
                echo -e "${RED}Error: mark requires a file path${NC}"
                echo ""
                usage
                exit 1
            fi
            cmd_mark "$2"
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            echo -e "${RED}Unknown command: $cmd${NC}"
            echo ""
            usage
            exit 1
            ;;
    esac
}

main "$@"
