use chant::spec::{Spec, SpecFrontmatter, SpecStatus};

pub struct SpecBuilder {
    id: String,
    status: SpecStatus,
    depends_on: Option<Vec<String>>,
}

impl SpecBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            status: SpecStatus::Pending,
            depends_on: None,
        }
    }

    pub fn with_status(mut self, status: SpecStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_depends_on(mut self, deps: Vec<String>) -> Self {
        self.depends_on = Some(deps);
        self
    }

    pub fn build(self) -> Spec {
        Spec {
            id: self.id,
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                status: self.status,
                depends_on: self.depends_on,
                labels: None,
                target_files: None,
                context: None,
                prompt: None,
                branch: None,
                commits: None,
                completed_at: None,
                model: None,
                tracks: None,
                informed_by: None,
                origin: None,
                schedule: None,
                source_branch: None,
                target_branch: None,
                conflicting_files: None,
                blocked_specs: None,
                original_spec: None,
                last_verified: None,
                verification_status: None,
                verification_failures: None,
                replayed_at: None,
                replay_count: None,
                original_completed_at: None,
                approval: None,
                members: None,
                output_schema: None,
                derived_fields: None,
                public: None,
                retry_state: None,
            },
            title: None,
            body: String::new(),
        }
    }
}
