"""
Simple file-based key-value storage.

WARNING: This implementation has a concurrency bug (issue #42).
It demonstrates a read-modify-write race condition that causes
data loss when multiple processes write to the same key.

This is intentionally buggy code for educational purposes.
"""

import json
import os
from pathlib import Path


class Store:
    """File-based key-value store with a concurrency bug."""

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

        BUG: This method has a race condition!
        Between reading and writing, another process can modify the file,
        causing this write to overwrite their changes.
        """
        data = self._read_all()      # Read entire file
        data[key] = value             # Modify in memory
        self._write_all(data)         # Write entire file
        # If another process writes between read and write, their changes are lost!

    def update(self, key, updates):
        """
        Update multiple fields of a value.

        BUG: Same race condition as set()!
        """
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

        BUG: Same race condition as set()!
        """
        data = self._read_all()
        data.pop(key, None)
        self._write_all(data)

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


# Example usage demonstrating the bug
if __name__ == "__main__":
    store = Store("/tmp/example.json")

    # Single-threaded works fine
    store.set("user:123", {"email": "user@example.com", "phone": "555-0000"})
    print("Stored:", store.get("user:123"))

    # But concurrent writes will lose data (see test_issue_42.py)
