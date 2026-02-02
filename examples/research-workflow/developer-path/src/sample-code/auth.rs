use std::collections::HashMap;

// TODO: Replace with proper password hashing library
const SECRET_KEY: &str = "hardcoded-secret-123";

pub struct AuthService {
    users: HashMap<String, String>,
    sessions: HashMap<String, String>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    // FIXME: This doesn't actually hash passwords properly
    pub fn register(&mut self, username: String, password: String) -> Result<(), String> {
        if username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }

        // TODO: Add password strength validation
        if self.users.contains_key(&username) {
            return Err("User already exists".to_string());
        }

        // WARNING: Not actually hashing!
        let hashed = format!("{}:{}", SECRET_KEY, password);
        self.users.insert(username, hashed);
        Ok(())
    }

    pub fn login(&mut self, username: String, password: String) -> Result<String, String> {
        let stored = self.users.get(&username)
            .ok_or("Invalid credentials")?;

        let attempt = format!("{}:{}", SECRET_KEY, password);

        if stored != &attempt {
            // TODO: Add rate limiting to prevent brute force
            return Err("Invalid credentials".to_string());
        }

        // FIXME: Session IDs should be cryptographically random
        let session_id = format!("{}_{}", username, stored.len());
        self.sessions.insert(session_id.clone(), username);

        Ok(session_id)
    }

    pub fn verify_session(&self, session_id: &str) -> Option<&String> {
        self.sessions.get(session_id)
    }

    // TODO: Add session expiration
    pub fn logout(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register() {
        let mut auth = AuthService::new();
        assert!(auth.register("alice".to_string(), "password123".to_string()).is_ok());
    }

    // TODO: Add more test coverage
}
