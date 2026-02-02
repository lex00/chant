# API Documentation

## Rate Limiting

All API endpoints are protected by rate limiting to prevent abuse and ensure fair usage of system resources.

### Rate Limit Headers

All API responses include the following rate limit headers:

- `RateLimit-Limit`: The maximum number of requests allowed in the current time window
- `RateLimit-Remaining`: The number of requests remaining in the current time window
- `RateLimit-Reset`: The Unix timestamp when the current rate limit window resets

When a rate limit is exceeded, the server returns a `429 Too Many Requests` response with a `Retry-After` header indicating how many seconds to wait before retrying.

### General Endpoints

**Rate Limit:** 100 requests per 15 minutes per IP address

Applied to general data endpoints such as:
- `GET /api/users`

### Authentication Endpoints

**Rate Limit:** 5 requests per 15 minutes per IP address

Applied to authentication endpoints to protect against brute force attacks:
- `POST /api/login`

### Exceeding Rate Limits

When a rate limit is exceeded, the API returns:

```json
{
  "error": "Too Many Requests",
  "message": "Too many requests from this IP, please try again later.",
  "retryAfter": 123
}
```

Or for authentication endpoints:

```json
{
  "error": "Too Many Requests",
  "message": "Too many login attempts. Please try again later.",
  "retryAfter": 123
}
```

The `retryAfter` field indicates the number of seconds to wait before retrying the request.

## Endpoints

### POST /api/login

Authenticates a user and establishes a session.

**Rate Limit:** 5 requests per 15 minutes per IP

**Request:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response (200):**
```json
{
  "success": true
}
```

**Response (429 - Rate Limited):**
```json
{
  "error": "Too Many Requests",
  "message": "Too many login attempts. Please try again later.",
  "retryAfter": 123
}
```

### GET /api/users

Retrieves a list of all users.

**Rate Limit:** 100 requests per 15 minutes per IP

**Response (200):**
```json
[
  {
    "id": "string",
    "name": "string",
    "email": "string"
  }
]
```

**Response (429 - Rate Limited):**
```json
{
  "error": "Too Many Requests",
  "message": "Too many requests from this IP, please try again later.",
  "retryAfter": 123
}
```
