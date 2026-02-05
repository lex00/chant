use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::spec::Spec;

use super::spec_repository::SpecRepository;

/// In-memory implementation of SpecRepository for testing.
pub struct InMemorySpecRepository {
    specs: HashMap<String, Spec>,
}

impl Default for InMemorySpecRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySpecRepository {
    /// Create a new empty InMemorySpecRepository.
    pub fn new() -> Self {
        Self {
            specs: HashMap::new(),
        }
    }

    /// Create a new InMemorySpecRepository with pre-populated specs.
    pub fn with_specs(specs: Vec<Spec>) -> Self {
        let mut map = HashMap::new();
        for spec in specs {
            map.insert(spec.id.clone(), spec);
        }
        Self { specs: map }
    }
}

impl SpecRepository for InMemorySpecRepository {
    fn load(&self, id: &str) -> Result<Spec> {
        self.specs
            .get(id)
            .cloned()
            .context(format!("Spec not found: {}", id))
    }

    fn save(&self, _spec: &Spec) -> Result<()> {
        // In-memory repository is immutable for simplicity in testing
        // If mutable behavior is needed, wrap in RefCell or Mutex
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Spec>> {
        Ok(self.specs.values().cloned().collect())
    }
}
