"""
Regression test for issue #42: Concurrent write data loss.

This test reproduces the bug where two concurrent writes to the same
key result in one write being lost. The test uses multiprocessing to
simulate the production environment with multiple API workers.
"""

import multiprocessing
import tempfile
import time
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "src"))

from storage.store import Store


def worker_update_email(storage_path, barrier):
    """Worker A: Update email field."""
    store = Store(storage_path)

    # Wait for all workers to be ready
    barrier.wait()

    # Update email field
    user = store.get("user:123") or {}
    user["email"] = "new@example.com"
    store.set("user:123", user)


def worker_update_phone(storage_path, barrier):
    """Worker B: Update phone field."""
    store = Store(storage_path)

    # Wait for all workers to be ready
    barrier.wait()

    # Update phone field
    user = store.get("user:123") or {}
    user["phone"] = "555-1234"
    store.set("user:123", user)


def test_concurrent_writes_both_persist():
    """
    Test that concurrent writes to different fields both persist.

    This test will FAIL with the current buggy implementation because
    of the read-modify-write race condition. One of the updates will
    be lost when both workers read the file before either writes.

    Once the bug is fixed (by adding proper locking), this test should PASS.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        storage_path = Path(tmpdir) / "test.json"

        # Initialize with a user record
        store = Store(storage_path)
        store.set("user:123", {"email": "old@example.com", "phone": "555-0000"})

        # Create barrier to synchronize workers
        barrier = multiprocessing.Barrier(2)

        # Start two workers that will update different fields concurrently
        proc_a = multiprocessing.Process(
            target=worker_update_email,
            args=(storage_path, barrier)
        )
        proc_b = multiprocessing.Process(
            target=worker_update_phone,
            args=(storage_path, barrier)
        )

        proc_a.start()
        proc_b.start()

        # Wait for both to complete
        proc_a.join(timeout=5)
        proc_b.join(timeout=5)

        # Verify both updates persisted
        final_user = store.get("user:123")

        # Both fields should be updated
        assert final_user is not None, "User record disappeared!"
        assert final_user["email"] == "new@example.com", \
            f"Email update lost! Expected 'new@example.com', got '{final_user.get('email')}'"
        assert final_user["phone"] == "555-1234", \
            f"Phone update lost! Expected '555-1234', got '{final_user.get('phone')}'"


def increment_counter(storage_path, worker_id, iterations):
    """Worker function to increment counter (must be at module level for pickling)."""
    store = Store(storage_path)
    for _ in range(iterations):
        counter = store.get("counter")
        counter["value"] += 1
        counter[f"worker_{worker_id}"] = True
        store.set("counter", counter)


def test_concurrent_writes_stress():
    """
    Stress test with many concurrent writers.

    This amplifies the race condition by using more workers and more iterations.
    Helps verify the fix holds under high concurrency.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        storage_path = Path(tmpdir) / "test_stress.json"
        store = Store(storage_path)

        # Initialize counter
        store.set("counter", {"value": 0})

        num_workers = 4
        iterations = 5
        expected_total = num_workers * iterations

        # Start workers
        processes = []
        for i in range(num_workers):
            proc = multiprocessing.Process(
                target=increment_counter,
                args=(storage_path, i, iterations)
            )
            proc.start()
            processes.append(proc)

        # Wait for all workers
        for proc in processes:
            proc.join(timeout=10)

        # Verify all increments persisted
        final_counter = store.get("counter")
        assert final_counter["value"] == expected_total, \
            f"Lost updates! Expected {expected_total}, got {final_counter['value']}"

        # Verify all workers recorded their presence
        for i in range(num_workers):
            assert f"worker_{i}" in final_counter, \
                f"Worker {i} update was completely lost!"


if __name__ == "__main__":
    # Run the test to see it fail with the buggy implementation
    print("Running test_concurrent_writes_both_persist...")
    try:
        test_concurrent_writes_both_persist()
        print("✓ Test PASSED (bug is fixed!)")
    except AssertionError as e:
        print(f"✗ Test FAILED (bug present): {e}")

    print("\nRunning test_concurrent_writes_stress...")
    try:
        test_concurrent_writes_stress()
        print("✓ Stress test PASSED (bug is fixed!)")
    except AssertionError as e:
        print(f"✗ Stress test FAILED (bug present): {e}")
