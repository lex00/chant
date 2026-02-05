use anyhow::Result;

use crate::spec::{load_all_specs, Spec};

/// A trait for loading and saving specs from a storage backend.
pub trait SpecRepository {
    /// Load a spec by its ID.
    fn load(&self, id: &str) -> Result<Spec>;

    /// Save a spec to storage.
    fn save(&self, spec: &Spec) -> Result<()>;

    /// List all specs from storage.
    fn list_all(&self) -> Result<Vec<Spec>>;
}

/// File-based implementation of SpecRepository.
pub struct FileSpecRepository {
    specs_dir: std::path::PathBuf,
}

impl FileSpecRepository {
    /// Create a new FileSpecRepository for the given specs directory.
    pub fn new(specs_dir: std::path::PathBuf) -> Self {
        Self { specs_dir }
    }

    /// Get the specs directory path.
    pub fn specs_dir(&self) -> &std::path::Path {
        &self.specs_dir
    }
}

impl SpecRepository for FileSpecRepository {
    fn load(&self, id: &str) -> Result<Spec> {
        let spec_path = self.specs_dir.join(format!("{}.md", id));
        Spec::load(&spec_path)
    }

    fn save(&self, spec: &Spec) -> Result<()> {
        let spec_path = self.specs_dir.join(format!("{}.md", spec.id));
        spec.save(&spec_path)
    }

    fn list_all(&self) -> Result<Vec<Spec>> {
        load_all_specs(&self.specs_dir)
    }
}
