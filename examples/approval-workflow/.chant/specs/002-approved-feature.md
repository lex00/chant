---
type: implementation
status: pending
prompt: standard
approval:
  required: true
  status: pending
---
# Add rate limiting to API endpoints

## Goal

Implement rate limiting across all public API endpoints to prevent abuse and ensure fair usage of system resources.

## Context

We've experienced several incidents where automated scrapers and bots overwhelmed the API. Rate limiting will protect against denial-of-service attacks and ensure legitimate users have consistent access.

## Acceptance Criteria

- [x] Install and configure rate limiting middleware (express-rate-limit or similar)
- [x] Apply rate limits to all public endpoints (100 requests per 15 minutes per IP)
- [x] Apply stricter limits to authentication endpoints (5 requests per 15 minutes per IP)
- [x] Add rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset)
- [x] Return 429 Too Many Requests with Retry-After header when limit exceeded
- [x] Document rate limits in API documentation
- [x] Add tests for rate limiting behavior

## Implementation Plan

1. Add rate limiting dependency to package.json
2. Create rate limiter configuration utility
3. Apply middleware to Express app with different rules for different route groups
4. Add custom error handler for rate limit errors
5. Update API docs with rate limit information

## Approval Discussion

This spec demonstrates the approved state. In a real workflow, approval would be granted here, allowing the spec to be executed. For this example, the spec is kept in pending state to prevent test execution.
