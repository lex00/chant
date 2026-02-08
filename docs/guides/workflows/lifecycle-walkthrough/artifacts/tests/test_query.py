"""
Tests for query functionality
"""
import tempfile
import os
from src.query import execute_query


def test_query_matches_pattern():
    """Test that query finds matching lines"""
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
        f.write("ERROR: database connection failed\n")
        f.write("INFO: starting service\n")
        f.write("ERROR: timeout occurred\n")
        f.flush()
        filename = f.name

    try:
        results = execute_query("ERROR", filename)
        assert len(results) == 2
        assert "database connection failed" in results[0]
        assert "timeout occurred" in results[1]
    finally:
        os.unlink(filename)


def test_query_no_matches():
    """Test query with no matches returns empty list"""
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
        f.write("INFO: everything is fine\n")
        f.flush()
        filename = f.name

    try:
        results = execute_query("ERROR", filename)
        assert len(results) == 0
    finally:
        os.unlink(filename)


def test_query_missing_file():
    """Test query on non-existent file returns empty list"""
    results = execute_query("ERROR", "/nonexistent/file.txt")
    assert len(results) == 0
