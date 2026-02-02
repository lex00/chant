// Example application code for the approval workflow demonstration
// This is minimal sample code to provide context for the specs

const express = require('express');
const { generalLimiter, authLimiter } = require('./rateLimiter');

const app = express();

// Current session-based authentication (spec 001 will refactor this)
app.use(session({
  store: new RedisStore(),
  secret: process.env.SESSION_SECRET
}));

// Rate limiting (spec 002 adds this)
// Apply stricter rate limiting to authentication endpoints
app.post('/api/login', authLimiter, async (req, res) => {
  // Authentication logic here
  req.session.userId = user.id;
  res.json({ success: true });
});

// Using ORM for database queries (spec 003 proposed changing this)
const User = require('./models/User');
// Apply general rate limiting to data endpoints
app.get('/api/users', generalLimiter, async (req, res) => {
  const users = await User.findAll();
  res.json(users);
});

// Error handler for rate limit errors (spec 002)
app.use((err, req, res, next) => {
  if (err.status === 429) {
    const retryAfter = Math.ceil((err.resetTime - Date.now()) / 1000);
    return res.status(429)
      .set('Retry-After', retryAfter.toString())
      .json({
        error: 'Too Many Requests',
        message: err.message,
        retryAfter: retryAfter
      });
  }
  next(err);
});

module.exports = app;
