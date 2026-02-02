use std::collections::HashMap;

pub struct Database {
    data: HashMap<String, String>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    // FIXME: This is vulnerable to SQL injection if we switch to real SQL
    pub fn query(&self, query: &str) -> Option<String> {
        // TODO: Implement actual query parsing
        // For now, just treat query as a key lookup
        let key = query.trim();
        self.data.get(key).cloned()
    }

    pub fn insert(&mut self, key: String, value: String) -> Result<(), String> {
        if key.is_empty() {
            return Err("Key cannot be empty".to_string());
        }

        // TODO: Add validation for value size limits
        self.data.insert(key, value);
        Ok(())
    }

    // TODO: Add update and delete operations

    // FIXME: This exposes internal structure
    pub fn get_all(&self) -> &HashMap<String, String> {
        &self.data
    }

    // TODO: Add transaction support
    pub fn batch_insert(&mut self, items: Vec<(String, String)>) {
        for (key, value) in items {
            // WARNING: Ignoring errors here
            let _ = self.insert(key, value);
        }
    }
}

// TODO: Add connection pooling
// TODO: Add error recovery mechanisms
// FIXME: No tests for this module!
