# Writing Tests with Chant

With test plans defined in specs, agents write tests before implementation code. This page shows the execution workflow.

## Executing a Test Spec

Marcus triggers the refund flow test spec:

```bash
chant work 2026-01-20-001-rfn
```

The agent reads the spec's acceptance criteria and writes tests for each case.

## Agent Execution Flow

```
1. Agent reads spec acceptance criteria
2. Agent reads context files (test patterns, fixtures)
3. Agent reads existing code to understand interfaces
4. Agent writes tests (all should fail initially)
5. Agent runs tests to confirm they fail (red)
6. Agent implements code to make tests pass (green)
7. Agent marks criteria as complete
```

### Execution Log

```bash
chant log 2026-01-20-001-rfn
```

```
[10:02] Reading spec 2026-01-20-001-rfn...
[10:02] Reading context: tdd-standards/test-patterns.md
[10:03] Analyzing refund.py interface...
[10:04] Creating test file: tests/payments/test_refund_flow.py
[10:05] Writing test: test_full_refund_on_completed_transaction
[10:05] Writing test: test_partial_refund_correct_remaining_balance
[10:06] Writing test: test_refund_to_original_payment_method
[10:06] Writing test: test_refund_confirmation_email_triggered
[10:07] Writing test: test_refund_under_100_auto_approved
[10:07] Writing test: test_refund_100_to_1000_requires_team_lead
[10:08] Writing test: test_refund_over_1000_requires_manager
[10:08] Writing test: test_refund_flagged_account_requires_fraud_review
[10:09] Creating test file: tests/payments/test_refund_edge_cases.py
[10:10] Writing 8 edge case tests...
[10:12] Running tests... 16 passed, 0 failed
[10:12] Checking coverage... refund.py: 87% (target: 85%) ✓
[10:13] Running flakiness check (10 iterations)...
[10:15] All iterations passed. No flaky tests detected.
[10:15] Marking acceptance criteria complete.
[10:16] Committing changes.
```

## Generated Test File

The agent produces a complete test file:

**File: `tests/payments/test_refund_flow.py`**

```python
"""Tests for payment refund flow.

Spec: 2026-01-20-001-rfn
"""

import pytest
from decimal import Decimal
from unittest.mock import Mock, patch

from payments.refund import RefundService
from payments.models import Transaction, RefundRequest
from tests.factories import TransactionFactory, UserFactory


class TestRefundHappyPath:
    """Happy path tests for refund processing."""

    def test_full_refund_on_completed_transaction(self, refund_service, completed_transaction):
        """Full refund on completed transaction succeeds."""
        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=completed_transaction.amount,
        )

        assert result.status == "completed"
        assert result.refunded_amount == completed_transaction.amount
        assert completed_transaction.remaining_balance == Decimal("0.00")

    def test_partial_refund_correct_remaining_balance(self, refund_service, completed_transaction):
        """Partial refund calculates remaining balance correctly."""
        original_amount = completed_transaction.amount
        refund_amount = Decimal("50.00")

        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=refund_amount,
        )

        assert result.status == "completed"
        assert result.refunded_amount == refund_amount
        assert completed_transaction.remaining_balance == original_amount - refund_amount

    def test_refund_to_original_payment_method(self, refund_service, completed_transaction):
        """Refund goes to original payment method."""
        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=completed_transaction.amount,
        )

        assert result.payment_method_id == completed_transaction.payment_method_id

    @patch("payments.notifications.send_refund_confirmation")
    def test_refund_confirmation_email_triggered(
        self, mock_send_email, refund_service, completed_transaction
    ):
        """Refund triggers confirmation email."""
        refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=completed_transaction.amount,
        )

        mock_send_email.assert_called_once_with(
            user_id=completed_transaction.user_id,
            refund_amount=completed_transaction.amount,
            transaction_id=completed_transaction.id,
        )


class TestRefundAuthorization:
    """Authorization level tests for refunds."""

    def test_refund_under_100_auto_approved(self, refund_service, small_transaction):
        """Refunds under $100 are auto-approved."""
        result = refund_service.process_refund(
            transaction_id=small_transaction.id,
            amount=Decimal("50.00"),
        )

        assert result.status == "completed"
        assert result.approval_level == "auto"

    def test_refund_100_to_1000_requires_team_lead(self, refund_service, medium_transaction):
        """Refunds $100-$1000 require team lead approval."""
        result = refund_service.process_refund(
            transaction_id=medium_transaction.id,
            amount=Decimal("500.00"),
        )

        assert result.status == "pending_approval"
        assert result.required_approval_level == "team_lead"

    def test_refund_over_1000_requires_manager(self, refund_service, large_transaction):
        """Refunds over $1000 require manager approval."""
        result = refund_service.process_refund(
            transaction_id=large_transaction.id,
            amount=Decimal("1500.00"),
        )

        assert result.status == "pending_approval"
        assert result.required_approval_level == "manager"

    def test_refund_flagged_account_requires_fraud_review(
        self, refund_service, flagged_user_transaction
    ):
        """Refunds on flagged accounts require fraud review."""
        result = refund_service.process_refund(
            transaction_id=flagged_user_transaction.id,
            amount=Decimal("50.00"),
        )

        assert result.status == "pending_fraud_review"


# ... additional tests in test_refund_edge_cases.py
```

## Parallel Test Execution

For the full payment coverage initiative, Marcus runs all test specs in parallel:

```bash
# Note: work in parallel by passing multiple spec IDs or using --chain
chant work 001-rfn 002-cur 003-frd 004-rty
```

```
Starting parallel execution (4 specs)

[001-rfn] Starting: Refund flow tests...
[002-cur] Starting: Currency conversion tests...
[003-frd] Starting: Fraud handling tests...
[004-rty] Starting: Retry logic tests...

[003-frd] Completed (3 tests added)
[004-rty] Completed (4 tests added)
[002-cur] Completed (4 tests added)
[001-rfn] Completed (16 tests added)

All 4 specs completed. 27 tests added.
Payment service coverage: 45% → 86%
```

## Monitoring Progress

While agents run, Marcus monitors progress:

```bash
# Watch live execution
chant log 2026-01-20-001-rfn

# Check all TDD specs
chant list --label tdd
```

```
ID           Type    Status       Title
───────────  ──────  ───────────  ────────────────────────────────
001-rfn      code    in_progress  Refund flow tests
002-cur      code    in_progress  Currency conversion tests
003-frd      code    completed    Fraud handling tests
004-rty      code    completed    Retry logic tests
```

## Merging Test Changes

After completion, merge all test changes:

```bash
chant merge --all-completed --rebase --auto
```

```
Merging 4 completed specs...

  001-rfn (refund tests):    Merged ✓
  002-cur (currency tests):  Merged ✓
  003-frd (fraud tests):     Merged ✓
  004-rty (retry tests):     Merged ✓

All specs merged to main.
```

## Before/After Metrics

After the test suite expansion:

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Payment coverage | 45% | 86% | 85% ✓ |
| Refund module | 38% | 87% | 85% ✓ |
| Flaky tests | 18% | 4% | <5% ✓ |
| Test count | 42 | 69 | — |

## What's Next

With tests written, see how to ensure consistency across teams:

**[Ensuring Quality](05-consistency.md)** — Enforcing test standards via configuration.
