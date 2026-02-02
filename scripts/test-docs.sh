#!/usr/bin/env bash
#
# test-docs.sh - Validate markdown links in documentation
#
# Usage:
#   ./scripts/test-docs.sh
#
# Scans docs/ and examples/ for markdown files and validates:
# - Relative links point to existing files
# - Reports broken links with file:line references
# - Exit 0 if all links valid, 1 if any broken

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_ROOT="$PROJECT_ROOT/docs"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track statistics
total_files=0
total_links=0
broken_links=0

# Process a single markdown file
check_file() {
    local file="$1"
    local line_num=0
    local file_has_broken_links=false

    ((total_files++))

    while IFS= read -r line; do
        ((line_num++))

        # Extract markdown links: [text](url)
        # This regex captures the URL part of markdown links
        while [[ "$line" =~ \[([^\]]+)\]\(([^\)]+)\) ]]; do
            local link_text="${BASH_REMATCH[1]}"
            local link_url="${BASH_REMATCH[2]}"

            # Remove the matched part to find next link in line
            line="${line#*\]($link_url)}"

            ((total_links++))

            # Skip non-relative links (http://, https://, mailto:, #anchors, etc.)
            if [[ "$link_url" =~ ^(https?://|mailto:|#) ]]; then
                continue
            fi

            # Handle anchor-only links within same file
            if [[ "$link_url" =~ ^# ]]; then
                continue
            fi

            # Remove anchor from URL if present
            local link_path="${link_url%%#*}"

            # Skip empty paths (just anchors)
            if [[ -z "$link_path" ]]; then
                continue
            fi

            # Skip directory-only links (ending with /)
            if [[ "$link_path" =~ /$ ]]; then
                continue
            fi

            # Resolve relative path
            local file_dir
            file_dir="$(dirname "$file")"

            local target_path=""

            # Resolve relative to current file directory (standard markdown behavior)
            if cd "$file_dir" 2>/dev/null; then
                # Use realpath to resolve .. and . in the path
                if [[ "$(uname)" == "Darwin" ]]; then
                    # macOS doesn't have realpath by default, use Python
                    target_path=$(python3 -c "import os; print(os.path.realpath('$link_path'))" 2>/dev/null || echo "")
                else
                    target_path=$(realpath "$link_path" 2>/dev/null || echo "")
                fi
                cd - >/dev/null
            fi

            # Check if target exists
            if [[ -z "$target_path" ]] || [[ ! -e "$target_path" ]]; then
                if [[ "$file_has_broken_links" == false ]]; then
                    echo -e "${RED}Broken links in $file:${NC}"
                    file_has_broken_links=true
                fi
                echo -e "  ${file}:${line_num}: $link_url"
                ((broken_links++))
            fi
        done
    done < "$file"
}

# Main execution
main() {
    echo "Checking documentation links..."
    echo ""

    # Find all markdown files in docs/ and examples/
    while IFS= read -r file; do
        check_file "$file"
    done < <(find "$PROJECT_ROOT/docs" "$PROJECT_ROOT/examples" -name "*.md" 2>/dev/null | sort)

    echo ""
    echo "=============================="
    echo "Files checked: $total_files"
    echo "Links checked: $total_links"

    if [[ $broken_links -eq 0 ]]; then
        echo -e "${GREEN}Broken links: 0${NC}"
        echo -e "${GREEN}All links valid!${NC}"
        exit 0
    else
        echo -e "${RED}Broken links: $broken_links${NC}"
        exit 1
    fi
}

main "$@"
