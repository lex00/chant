# Acme Test Patterns

## Naming Convention

Tests follow: `test_<action>_<condition>_<expected_result>`

### Good Examples

```python
def test_refund_exceeds_original_returns_400()
def test_login_invalid_password_increments_failure_count()
def test_subscription_expired_blocks_api_access()
def test_partial_refund_correct_remaining_balance()
def test_currency_conversion_on_international_refund()
```

### Bad Examples

```python
def test_refund()  # Too vague
def test_login_works()  # Doesn't describe condition
def test_edge_case_1()  # Not descriptive
def test_refund_flow()  # Too broad
```

## Required Assertions

Every test must assert:

1. **Return value or side effect** - What the function produces
2. **State change** (if applicable) - How the system state changed
3. **Audit/logging** (for security-sensitive operations) - What was recorded

### Example

```python
def test_refund_exceeds_original_returns_400(refund_service, transaction):
    """Refund amount exceeding original transaction is rejected."""
    excessive_amount = transaction.amount + Decimal("100.00")

    result = refund_service.process_refund(
        transaction_id=transaction.id,
        amount=excessive_amount,
    )

    # 1. Return value
    assert result.status == "rejected"
    assert result.error_code == "REFUND_EXCEEDS_ORIGINAL"
    assert result.http_status == 400

    # 2. State change
    assert transaction.remaining_balance == transaction.amount  # Unchanged

    # 3. Audit (for financial operations)
    assert audit_log.contains(
        user_id=transaction.user_id,
        action="refund_rejected",
        reason="amount_exceeds_original",
    )
```

## Fixture Requirements

### Use Factories, Not Raw Fixtures

**Good:**
```python
@pytest.fixture
def completed_transaction(transaction_factory):
    return transaction_factory(
        status="completed",
        amount=Decimal("100.00"),
    )
```

**Bad:**
```python
@pytest.fixture
def completed_transaction():
    return Transaction(
        id="trans_123",
        user_id="user_456",
        amount=Decimal("100.00"),
        status="completed",
        created_at=datetime(2026, 1, 1),
        # ... 20 more fields hardcoded
    )
```

### Mock External Services at Boundary

Mock external services (payment processors, email services) at the client boundary, not deep in the implementation.

**Good:**
```python
@patch("payments.processor.PaymentProcessor.refund")
def test_refund_processor_timeout(mock_refund, refund_service, transaction):
    mock_refund.side_effect = TimeoutError("Request timed out")
    # Test retry logic
```

**Bad:**
```python
@patch("requests.post")  # Too low-level
def test_refund_processor_timeout(mock_post, refund_service, transaction):
    # Now tightly coupled to HTTP implementation
```

### Clean Up Database State

Use database transactions or explicit cleanup:

```python
@pytest.fixture(autouse=True)
def db_cleanup():
    yield
    db.session.rollback()
    db.session.remove()
```

## Test Organization

### Group by Category

Organize tests into classes by category:

```python
class TestRefundHappyPath:
    """Happy path tests for refund processing."""

class TestRefundAuthorization:
    """Authorization level tests for refunds."""

class TestRefundEdgeCases:
    """Edge case tests for refund processing."""

class TestRefundErrorHandling:
    """Error handling tests for refund processing."""
```

### One Assertion Category Per Test

Each test should focus on one behavior:

**Good:**
```python
def test_refund_confirmation_email_triggered(mock_email, refund_service, transaction):
    """Refund triggers confirmation email."""
    refund_service.process_refund(transaction.id, transaction.amount)

    mock_email.assert_called_once_with(
        user_id=transaction.user_id,
        refund_amount=transaction.amount,
    )
```

**Bad:**
```python
def test_refund_flow(mock_email, refund_service, transaction):
    """Test the entire refund flow."""
    # Tests 10 different things - hard to debug when it fails
```

## Docstrings

Every test must have a docstring explaining the behavior:

```python
def test_multiple_partial_refunds_exceeding_original_rejected(refund_service, transaction):
    """Multiple partial refunds cannot exceed original amount."""
```

## Parametrized Tests

Use parametrization for similar test cases:

```python
@pytest.mark.parametrize("amount,approval_level", [
    (Decimal("50.00"), "auto"),
    (Decimal("500.00"), "team_lead"),
    (Decimal("1500.00"), "manager"),
])
def test_refund_authorization_levels(refund_service, transaction, amount, approval_level):
    """Refunds require appropriate authorization based on amount."""
    result = refund_service.process_refund(transaction.id, amount)
    assert result.required_approval_level == approval_level
```

## Anti-Patterns

### Don't Test Implementation Details

**Bad:**
```python
def test_refund_calls_validate_then_process():
    """Test internal method call order."""
    # Brittle - breaks when refactoring
```

**Good:**
```python
def test_refund_exceeds_original_returns_400():
    """Refund amount exceeding original is rejected."""
    # Tests behavior, not implementation
```

### Don't Use Sleep for Timing

**Bad:**
```python
def test_async_refund_processing():
    refund_service.process_async(transaction.id)
    time.sleep(2)  # Hope it's done
    assert transaction.status == "refunded"
```

**Good:**
```python
def test_async_refund_processing():
    result = await refund_service.process_async(transaction.id)
    assert result.status == "refunded"
```

### Don't Share State Between Tests

Each test must be independent. Use fixtures, not class attributes:

**Bad:**
```python
class TestRefunds:
    transaction = create_transaction()  # Shared state
```

**Good:**
```python
class TestRefunds:
    @pytest.fixture
    def transaction(self):
        return create_transaction()
```
