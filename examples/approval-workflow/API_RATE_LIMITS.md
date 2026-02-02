# API Rate Limiting

This API implements rate limiting to protect against abuse and ensure fair usage of system resources. All endpoints are subject to rate limiting based on the client's IP address.

## Rate Limit Tiers

### General Public Endpoints

**Limit:** 100 requests per 15 minutes per IP

Applies to:
- `GET /api/users` - Fetch all users

**Response Headers:**
```
RateLimit-Limit: 100
RateLimit-Remaining: 99
RateLimit-Reset: 1643750400
```

### Authentication Endpoints (Strict)

**Limit:** 5 requests per 15 minutes per IP

Applies to:
- `POST /api/login` - User authentication

This stricter limit protects against brute force attacks and credential stuffing.

**Special Behavior:** Successful requests do not count against the rate limit quota.

**Response Headers:**
```
RateLimit-Limit: 5
RateLimit-Remaining: 4
RateLimit-Reset: 1643750400
```

## Rate Limit Exceeded

When you exceed the rate limit, the API responds with HTTP status code **429 Too Many Requests**:

```json
{
  "error": "Too Many Requests",
  "message": "You have exceeded the rate limit. Please try again later.",
  "retryAfter": 1643750400
}
```

For authentication endpoints specifically:

```json
{
  "error": "Too Many Requests",
  "message": "Too many login attempts. Please try again after some time.",
  "retryAfter": 1643750400
}
```

## Headers

### RateLimit Headers

The API includes the following standard rate limit headers in all responses:

- **RateLimit-Limit**: The maximum number of requests allowed in the current window
- **RateLimit-Remaining**: The number of requests remaining in the current window
- **RateLimit-Reset**: Unix timestamp when the current window resets

### Retry-After Header

When rate limited (429 response), the `retryAfter` field in the JSON response body indicates when you should retry your request.

## Best Practices

1. **Monitor Headers**: Check the `RateLimit-Remaining` header to track your usage
2. **Implement Backoff**: Use exponential backoff when you receive a 429 response
3. **Cache Results**: Cache API responses when possible to reduce request frequency
4. **Batch Operations**: Combine multiple operations in single requests where possible
5. **Distribute Load**: Spread requests over time rather than sending them in bursts

## Example

```bash
# Make a request and check rate limit headers
curl -i https://api.example.com/api/users

# Response includes:
# HTTP/1.1 200 OK
# RateLimit-Limit: 100
# RateLimit-Remaining: 99
# RateLimit-Reset: 1643750400
```

## Questions?

For more information about our API, please refer to the main API documentation.
