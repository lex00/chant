const request = require('supertest');
const express = require('express');
const { generalLimiter, authLimiter } = require('../src/rateLimiter');

describe('Rate Limiting', () => {
  let app;

  beforeEach(() => {
    app = express();

    // Test endpoint with auth limiter
    app.post('/api/login', authLimiter, (req, res) => {
      res.json({ success: true });
    });

    // Test endpoint with general limiter
    app.get('/api/users', generalLimiter, (req, res) => {
      res.json([]);
    });
  });

  describe('Auth Limiter', () => {
    it('should allow 5 requests within the window', async () => {
      for (let i = 0; i < 5; i++) {
        const res = await request(app)
          .post('/api/login')
          .expect(200);

        expect(res.headers['ratelimit-limit']).toBe('5');
        expect(res.headers['ratelimit-remaining']).toBe(String(4 - i));
      }
    });

    it('should return 429 on 6th request within the window', async () => {
      // Make 5 allowed requests
      for (let i = 0; i < 5; i++) {
        await request(app)
          .post('/api/login')
          .expect(200);
      }

      // 6th request should be rejected
      const res = await request(app)
        .post('/api/login')
        .expect(429);

      expect(res.body.error).toBe('Too Many Requests');
      expect(res.body.message).toContain('login attempts');
      expect(res.headers['retry-after']).toBeDefined();
    });

    it('should include Retry-After header on rate limit exceeded', async () => {
      // Make 5 requests
      for (let i = 0; i < 5; i++) {
        await request(app)
          .post('/api/login')
          .expect(200);
      }

      // 6th request should have Retry-After header
      const res = await request(app)
        .post('/api/login')
        .expect(429);

      const retryAfter = res.headers['retry-after'];
      expect(retryAfter).toBeDefined();
      expect(Number(retryAfter)).toBeGreaterThan(0);
    });

    it('should include RateLimit headers in response', async () => {
      const res = await request(app)
        .post('/api/login')
        .expect(200);

      expect(res.headers['ratelimit-limit']).toBe('5');
      expect(res.headers['ratelimit-remaining']).toBeDefined();
      expect(res.headers['ratelimit-reset']).toBeDefined();
    });
  });

  describe('General Limiter', () => {
    it('should allow 100 requests within the window', async () => {
      for (let i = 0; i < 100; i++) {
        const res = await request(app)
          .get('/api/users')
          .expect(200);

        expect(res.headers['ratelimit-limit']).toBe('100');
        expect(res.headers['ratelimit-remaining']).toBe(String(99 - i));
      }
    });

    it('should return 429 on 101st request within the window', async () => {
      // Make 100 allowed requests
      for (let i = 0; i < 100; i++) {
        await request(app)
          .get('/api/users')
          .expect(200);
      }

      // 101st request should be rejected
      const res = await request(app)
        .get('/api/users')
        .expect(429);

      expect(res.body.error).toBe('Too Many Requests');
      expect(res.headers['retry-after']).toBeDefined();
    });

    it('should include RateLimit headers on general endpoints', async () => {
      const res = await request(app)
        .get('/api/users')
        .expect(200);

      expect(res.headers['ratelimit-limit']).toBe('100');
      expect(res.headers['ratelimit-remaining']).toBeDefined();
      expect(res.headers['ratelimit-reset']).toBeDefined();
    });
  });

  describe('Rate Limit Headers', () => {
    it('should return correct X-RateLimit headers format', async () => {
      const res = await request(app)
        .get('/api/users')
        .expect(200);

      // Standard headers (express-rate-limit with standardHeaders: true)
      expect(res.headers['ratelimit-limit']).toBeDefined();
      expect(res.headers['ratelimit-remaining']).toBeDefined();
      expect(res.headers['ratelimit-reset']).toBeDefined();

      // Should be parseable integers
      expect(Number(res.headers['ratelimit-limit'])).toBeGreaterThan(0);
      expect(Number(res.headers['ratelimit-remaining'])).toBeGreaterThanOrEqual(0);
      expect(Number(res.headers['ratelimit-reset'])).toBeGreaterThan(0);
    });

    it('should decrease RateLimit-Remaining with each request', async () => {
      const res1 = await request(app)
        .get('/api/users')
        .expect(200);

      const remaining1 = Number(res1.headers['ratelimit-remaining']);

      const res2 = await request(app)
        .get('/api/users')
        .expect(200);

      const remaining2 = Number(res2.headers['ratelimit-remaining']);

      expect(remaining2).toBeLessThan(remaining1);
    });
  });

  describe('Different IP Addresses', () => {
    it('should track limits separately per IP', async () => {
      // First IP makes 5 requests on auth endpoint
      for (let i = 0; i < 5; i++) {
        await request(app)
          .post('/api/login')
          .set('X-Forwarded-For', '192.168.1.1')
          .expect(200);
      }

      // First IP should be rate limited
      await request(app)
        .post('/api/login')
        .set('X-Forwarded-For', '192.168.1.1')
        .expect(429);

      // Different IP should still have requests available
      const res = await request(app)
        .post('/api/login')
        .set('X-Forwarded-For', '192.168.1.2')
        .expect(200);

      expect(res.headers['ratelimit-remaining']).toBe('4');
    });
  });
});
