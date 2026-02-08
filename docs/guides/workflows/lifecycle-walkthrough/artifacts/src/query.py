"""
Query execution logic for datalog
"""
import re
from typing import List


def execute_query(pattern: str, filename: str) -> List[str]:
    """
    Execute a query pattern against a log file

    Args:
        pattern: Regex pattern to search for
        filename: Path to log file

    Returns:
        List of matching lines
    """
    results = []
    try:
        with open(filename, 'r') as f:
            for line in f:
                if re.search(pattern, line):
                    results.append(line.rstrip('\n'))
    except FileNotFoundError:
        pass

    return results
