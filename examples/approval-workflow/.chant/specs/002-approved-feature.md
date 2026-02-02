---
type: implementation
status: pending
prompt: standard
approval:
  required: true
  status: approved
  by: alice
  at: 2026-02-01T09:15:23Z
---
# Add rate limiting to API endpoints

## Goal

Implement rate limiting across all public API endpoints to prevent abuse and ensure fair usage of system resources.

## Context

We've experienced several incidents where automated scrapers and bots overwhelmed the API. Rate limiting will protect against denial-of-service attacks and ensure legitimate users have consistent access.

## Acceptance Criteria

- [ ] Install and configure rate limiting middleware (express-rate-limit or similar)
- [ ] Apply rate limits to all public endpoints (100 requests per 15 minutes per IP)
- [ ] Apply stricter limits to authentication endpoints (5 requests per 15 minutes per IP)
- [ ] Add rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset)
- [ ] Return 429 Too Many Requests with Retry-After header when limit exceeded
- [ ] Document rate limits in API documentation
- [ ] Add tests for rate limiting behavior

## Implementation Plan

1. Add rate limiting dependency to package.json
2. Create rate limiter configuration utility
3. Apply middleware to Express app with different rules for different route groups
4. Add custom error handler for rate limit errors
5. Update API docs with rate limit information

## Approval Discussion

**alice** - 2026-02-01 09:15 - APPROVED

Reviewed the implementation plan and acceptance criteria. The rate limits seem reasonable and the approach is sound. Approved to proceed with implementation.

Key points:
- 100 req/15min for general endpoints is appropriate for our traffic
- 5 req/15min for auth protects against brute force attempts
- Good choice to use standard rate limit headers
- Make sure we test the Retry-After header calculation

