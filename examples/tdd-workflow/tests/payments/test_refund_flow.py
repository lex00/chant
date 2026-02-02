"""Tests for payment refund flow.

Spec: 002-test-suite-driver.1

This file demonstrates the test output from the TDD workflow example.
Tests are organized by category as defined in the spec's acceptance criteria.
"""

import pytest
from decimal import Decimal
from unittest.mock import Mock, patch

from payments.refund import RefundService, Transaction, RefundResult


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


class TestRefundEdgeCases:
    """Edge case tests for refund processing."""

    def test_refund_older_than_90_days_rejected(self, refund_service, old_transaction):
        """Refunds on transactions older than 90 days are rejected."""
        result = refund_service.process_refund(
            transaction_id=old_transaction.id,
            amount=old_transaction.amount,
        )

        assert result.status == "rejected"
        assert result.error_code == "TRANSACTION_TOO_OLD"

    def test_refund_exceeds_original_returns_400(self, refund_service, completed_transaction):
        """Refund amount exceeding original transaction is rejected."""
        excessive_amount = completed_transaction.amount + Decimal("100.00")

        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=excessive_amount,
        )

        assert result.status == "rejected"
        assert result.error_code == "REFUND_EXCEEDS_ORIGINAL"

    def test_multiple_partial_refunds_exceeding_original_rejected(
        self, refund_service, partially_refunded_transaction
    ):
        """Multiple partial refunds cannot exceed original amount."""
        # Transaction already has $60 refunded, original was $100
        result = refund_service.process_refund(
            transaction_id=partially_refunded_transaction.id,
            amount=Decimal("50.00"),  # Would total $110
        )

        assert result.status == "rejected"
        assert result.error_code == "REFUND_EXCEEDS_ORIGINAL"

    def test_refund_on_disputed_transaction_blocked(
        self, refund_service, disputed_transaction
    ):
        """Refunds on disputed transactions are blocked."""
        result = refund_service.process_refund(
            transaction_id=disputed_transaction.id,
            amount=disputed_transaction.amount,
        )

        assert result.status == "blocked"
        assert result.error_code == "DISPUTED_TRANSACTION"


class TestRefundErrorHandling:
    """Error handling tests for refund processing."""

    def test_invalid_transaction_id_returns_404(self, refund_service):
        """Invalid transaction ID returns 404."""
        result = refund_service.process_refund(
            transaction_id="nonexistent-id",
            amount=Decimal("50.00"),
        )

        assert result.status == "error"
        assert result.error_code == "TRANSACTION_NOT_FOUND"
        assert result.http_status == 404

    def test_insufficient_balance_returns_400(
        self, refund_service, fully_refunded_transaction
    ):
        """Insufficient balance returns 400 with clear message."""
        result = refund_service.process_refund(
            transaction_id=fully_refunded_transaction.id,
            amount=Decimal("1.00"),
        )

        assert result.status == "error"
        assert result.error_code == "INSUFFICIENT_BALANCE"
        assert result.http_status == 400
        assert "remaining balance" in result.message.lower()

    @patch("payments.processor.PaymentProcessor.refund")
    def test_processor_timeout_triggers_retry(
        self, mock_processor, refund_service, completed_transaction
    ):
        """Payment processor timeout triggers retry with backoff."""
        mock_processor.side_effect = TimeoutError("Request timed out")

        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=completed_transaction.amount,
        )

        assert result.status == "queued_for_retry"
        assert result.retry_delay_seconds == 30  # Initial backoff

    @patch("payments.database.RefundRepository.save")
    def test_database_error_triggers_rollback(
        self, mock_save, refund_service, completed_transaction
    ):
        """Database error triggers rollback, no partial state."""
        mock_save.side_effect = Exception("Database connection lost")

        result = refund_service.process_refund(
            transaction_id=completed_transaction.id,
            amount=completed_transaction.amount,
        )

        assert result.status == "error"
        assert result.error_code == "INTERNAL_ERROR"
        # Verify transaction state unchanged
        assert completed_transaction.remaining_balance == completed_transaction.amount
