// Example application code for the approval workflow demonstration
// This is minimal sample code to provide context for the specs

const express = require('express');
const app = express();

// Current session-based authentication (spec 001 will refactor this)
app.use(session({
  store: new RedisStore(),
  secret: process.env.SESSION_SECRET
}));

// No rate limiting yet (spec 002 will add this)
app.post('/api/login', async (req, res) => {
  // Authentication logic here
  req.session.userId = user.id;
  res.json({ success: true });
});

// Using ORM for database queries (spec 003 proposed changing this)
const User = require('./models/User');
app.get('/api/users', async (req, res) => {
  const users = await User.findAll();
  res.json(users);
});

module.exports = app;
