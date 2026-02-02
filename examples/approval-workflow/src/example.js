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

// Apply rate limiting middleware
// Stricter limits for authentication endpoints (5 req/15 min per IP)
app.post('/api/login', authLimiter, async (req, res) => {
  // Authentication logic here
  req.session.userId = user.id;
  res.json({ success: true });
});

// General rate limiting for public endpoints (100 req/15 min per IP)
// Using ORM for database queries (spec 003 proposed changing this)
const User = require('./models/User');
app.get('/api/users', generalLimiter, async (req, res) => {
  const users = await User.findAll();
  res.json(users);
});

module.exports = app;
