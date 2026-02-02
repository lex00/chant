"""
Simple file-based key-value storage.

FIXED: Issue #42 - Added file locking to prevent concurrent write data loss.
Uses fcntl-based locking to ensure atomic read-modify-write operations.
"""

import fcntl
import json
import os
from pathlib import Path


class Store:
    """File-based key-value store with proper concurrency control."""

    def __init__(self, storage_path):
        """Initialize store with a file path."""
        self.storage_path = Path(storage_path)
        self.storage_path.parent.mkdir(parents=True, exist_ok=True)
        if not self.storage_path.exists():
            self._write_all({})

    def get(self, key):
        """Get a value from the store."""
        data = self._read_all()
        return data.get(key)

    def set(self, key, value):
        """
        Set a value in the store.

        Uses file locking to ensure atomic read-modify-write operation.
        Multiple processes can safely write concurrently without data loss.
        """
        with self._lock():
            data = self._read_all()
            data[key] = value
            self._write_all(data)

    def update(self, key, updates):
        """
        Update multiple fields of a value.

        Uses file locking to ensure atomic read-modify-write operation.
        """
        with self._lock():
            data = self._read_all()
            if key in data:
                if isinstance(data[key], dict):
                    data[key].update(updates)
                else:
                    data[key] = updates
            else:
                data[key] = updates
            self._write_all(data)

    def delete(self, key):
        """
        Delete a key from the store.

        Uses file locking to ensure atomic read-modify-write operation.
        """
        with self._lock():
            data = self._read_all()
            data.pop(key, None)
            self._write_all(data)

    def increment(self, key, field, amount=1):
        """
        Atomically increment a numeric field.

        Uses file locking to ensure atomic read-modify-write operation.
        """
        with self._lock():
            data = self._read_all()
            if key not in data:
                data[key] = {}
            if not isinstance(data[key], dict):
                raise ValueError(f"Cannot increment field on non-dict value for key {key}")
            data[key][field] = data[key].get(field, 0) + amount
            self._write_all(data)
            return data[key][field]

    def _read_all(self):
        """Read the entire storage file."""
        if not self.storage_path.exists():
            return {}
        with open(self.storage_path, 'r') as f:
            return json.load(f)

    def _write_all(self, data):
        """Write the entire storage file."""
        # Write to temp file first, then rename for atomicity
        temp_path = self.storage_path.with_suffix('.tmp')
        with open(temp_path, 'w') as f:
            json.dump(data, f, indent=2)
        temp_path.replace(self.storage_path)

    def _lock(self):
        """
        Context manager for file locking.

        Uses fcntl-based exclusive lock on a separate lock file to coordinate
        read-modify-write operations across processes. The lock ensures that
        only one process can perform a read-modify-write cycle at a time,
        preventing the race condition described in issue #42.
        """
        return _FileLock(self.storage_path.with_suffix('.lock'))


class _FileLock:
    """Context manager for fcntl-based file locking."""

    def __init__(self, lock_path):
        self.lock_path = lock_path
        self.lock_file = None

    def __enter__(self):
        self.lock_path.parent.mkdir(parents=True, exist_ok=True)
        self.lock_file = open(self.lock_path, 'w')
        fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_EX)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.lock_file:
            fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_UN)
            self.lock_file.close()


# Example usage demonstrating the bug
if __name__ == "__main__":
    store = Store("/tmp/example.json")

    # Single-threaded works fine
    store.set("user:123", {"email": "user@example.com", "phone": "555-0000"})
    print("Stored:", store.get("user:123"))

    # But concurrent writes will lose data (see test_issue_42.py)
