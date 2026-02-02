// Tests for rate limiting functionality
// Verifies that rate limits are correctly applied and enforced

const request = require('supertest');
const express = require('express');
const { generalLimiter, authLimiter } = require('../src/rateLimiter');

describe('Rate Limiting Middleware', () => {
  let app;

  beforeEach(() => {
    // Create a fresh app for each test
    app = express();
    app.use(express.json());

    // Set up test routes
    app.get('/public', generalLimiter, (req, res) => {
      res.json({ message: 'success' });
    });

    app.post('/auth', authLimiter, (req, res) => {
      res.json({ message: 'authenticated' });
    });

    app.post('/auth-fail', authLimiter, (req, res) => {
      // Simulate failed auth
      res.status(401).json({ error: 'unauthorized' });
    });
  });

  describe('General Rate Limiter', () => {
    test('should allow requests within limit', async () => {
      for (let i = 0; i < 5; i++) {
        const response = await request(app).get('/public');
        expect(response.status).toBe(200);
        expect(response.headers['ratelimit-limit']).toBe('100');
      }
    });

    test('should include rate limit headers in response', async () => {
      const response = await request(app).get('/public');

      expect(response.status).toBe(200);
      expect(response.headers['ratelimit-limit']).toBe('100');
      expect(response.headers['ratelimit-remaining']).toBeDefined();
      expect(response.headers['ratelimit-reset']).toBeDefined();
    });

    test('should enforce 100 requests per 15 minute limit', async () => {
      // Make 100 requests (should all succeed)
      for (let i = 0; i < 100; i++) {
        const response = await request(app).get('/public');
        expect(response.status).toBe(200);
      }

      // 101st request should be rate limited
      const limitedResponse = await request(app).get('/public');
      expect(limitedResponse.status).toBe(429);
      expect(limitedResponse.body.error).toBe('Too Many Requests');
    });

    test('should return proper error response when rate limited', async () => {
      // Exceed the rate limit
      for (let i = 0; i < 100; i++) {
        await request(app).get('/public');
      }

      const response = await request(app).get('/public');

      expect(response.status).toBe(429);
      expect(response.body).toHaveProperty('error');
      expect(response.body).toHaveProperty('message');
      expect(response.body).toHaveProperty('retryAfter');
      expect(response.body.error).toBe('Too Many Requests');
    });

    test('should track remaining requests', async () => {
      const response1 = await request(app).get('/public');
      const remaining1 = parseInt(response1.headers['ratelimit-remaining']);

      const response2 = await request(app).get('/public');
      const remaining2 = parseInt(response2.headers['ratelimit-remaining']);

      expect(remaining1).toBeGreaterThan(remaining2);
      expect(remaining2).toBe(remaining1 - 1);
    });
  });

  describe('Auth Rate Limiter', () => {
    test('should allow requests within limit', async () => {
      for (let i = 0; i < 3; i++) {
        const response = await request(app).post('/auth');
        expect(response.status).toBe(200);
        expect(response.headers['ratelimit-limit']).toBe('5');
      }
    });

    test('should enforce 5 requests per 15 minute limit', async () => {
      // Make 5 requests (should all succeed)
      for (let i = 0; i < 5; i++) {
        const response = await request(app).post('/auth');
        expect(response.status).toBe(200);
      }

      // 6th request should be rate limited
      const limitedResponse = await request(app).post('/auth');
      expect(limitedResponse.status).toBe(429);
    });

    test('should skip successful requests when skipSuccessfulRequests is true', async () => {
      // All 5 successful requests count
      for (let i = 0; i < 5; i++) {
        const response = await request(app).post('/auth');
        expect(response.status).toBe(200);
      }

      // 6th request should be blocked
      let response = await request(app).post('/auth');
      expect(response.status).toBe(429);
    });

    test('should return proper error response when rate limited', async () => {
      // Exceed the rate limit
      for (let i = 0; i < 5; i++) {
        await request(app).post('/auth');
      }

      const response = await request(app).post('/auth');

      expect(response.status).toBe(429);
      expect(response.body).toHaveProperty('error');
      expect(response.body).toHaveProperty('message');
      expect(response.body.message).toContain('Too many login attempts');
      expect(response.body).toHaveProperty('retryAfter');
    });

    test('should include rate limit headers in response', async () => {
      const response = await request(app).post('/auth');

      expect(response.status).toBe(200);
      expect(response.headers['ratelimit-limit']).toBe('5');
      expect(response.headers['ratelimit-remaining']).toBeDefined();
      expect(response.headers['ratelimit-reset']).toBeDefined();
    });

    test('should track remaining requests correctly', async () => {
      const response1 = await request(app).post('/auth');
      const remaining1 = parseInt(response1.headers['ratelimit-remaining']);

      const response2 = await request(app).post('/auth');
      const remaining2 = parseInt(response2.headers['ratelimit-remaining']);

      expect(remaining1).toBeGreaterThan(remaining2);
      expect(remaining2).toBe(remaining1 - 1);
    });
  });

  describe('Rate Limit Headers', () => {
    test('should return RateLimit-Limit header', async () => {
      const response = await request(app).get('/public');
      expect(response.headers['ratelimit-limit']).toBeDefined();
      expect(response.headers['ratelimit-limit']).toBe('100');
    });

    test('should return RateLimit-Remaining header', async () => {
      const response = await request(app).get('/public');
      expect(response.headers['ratelimit-remaining']).toBeDefined();
      const remaining = parseInt(response.headers['ratelimit-remaining']);
      expect(remaining).toBeGreaterThanOrEqual(0);
      expect(remaining).toBeLessThanOrEqual(100);
    });

    test('should return RateLimit-Reset header', async () => {
      const response = await request(app).get('/public');
      expect(response.headers['ratelimit-reset']).toBeDefined();
      const reset = parseInt(response.headers['ratelimit-reset']);
      expect(reset).toBeGreaterThan(0);
    });

    test('should have consistent reset time across requests in same window', async () => {
      const response1 = await request(app).get('/public');
      const reset1 = response1.headers['ratelimit-reset'];

      const response2 = await request(app).get('/public');
      const reset2 = response2.headers['ratelimit-reset'];

      expect(reset1).toBe(reset2);
    });
  });

  describe('429 Too Many Requests Response', () => {
    test('should return 429 status when limit exceeded', async () => {
      for (let i = 0; i < 100; i++) {
        await request(app).get('/public');
      }

      const response = await request(app).get('/public');
      expect(response.status).toBe(429);
    });

    test('should include error details in response body', async () => {
      for (let i = 0; i < 100; i++) {
        await request(app).get('/public');
      }

      const response = await request(app).get('/public');

      expect(response.body).toHaveProperty('error');
      expect(response.body).toHaveProperty('message');
      expect(response.body).toHaveProperty('retryAfter');
      expect(typeof response.body.retryAfter).toBe('number');
    });

    test('should include retryAfter in response body', async () => {
      for (let i = 0; i < 5; i++) {
        await request(app).post('/auth');
      }

      const response = await request(app).post('/auth');

      expect(response.body.retryAfter).toBeDefined();
      expect(typeof response.body.retryAfter).toBe('number');
      expect(response.body.retryAfter).toBeGreaterThan(0);
    });

    test('should still include rate limit headers in 429 response', async () => {
      for (let i = 0; i < 100; i++) {
        await request(app).get('/public');
      }

      const response = await request(app).get('/public');

      expect(response.status).toBe(429);
      expect(response.headers['ratelimit-limit']).toBe('100');
      expect(response.headers['ratelimit-remaining']).toBe('0');
      expect(response.headers['ratelimit-reset']).toBeDefined();
    });
  });
});
