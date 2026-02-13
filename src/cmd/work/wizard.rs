//! Interactive wizard for selecting specs and prompts

use anyhow::Result;
use dialoguer::Select;
use std::path::Path;

use chant::spec::{Spec, SpecType};

/// Result of the wizard selection
pub enum WizardSelection {
    /// Run a single spec
    SingleSpec {
        spec_id: String,
        prompt: String,
        create_branch: bool,
    },
    /// Run all ready specs in parallel
    Parallel,
}

/// Run the interactive wizard for selecting a spec
pub fn run_wizard(specs_dir: &Path, prompts_dir: &Path) -> Result<Option<WizardSelection>> {
    // Load ready specs
    let ready_specs = super::load_ready_specs(specs_dir)?;

    if ready_specs.is_empty() {
        println!("No ready specs to execute.");
        return Ok(None);
    }

    // Build spec selection items
    let spec_items: Vec<String> = ready_specs
        .iter()
        .map(|s| {
            if let Some(title) = &s.title {
                format!("{}  {}", s.id, title)
            } else {
                s.id.clone()
            }
        })
        .collect();

    // Add parallel option at the end
    let mut all_items = spec_items.clone();
    all_items.push("[Run all ready specs in parallel]".to_string());

    // Show spec selection
    let selection = Select::new()
        .with_prompt("? Select spec to work")
        .items(&all_items)
        .default(0)
        .interact()?;

    // Check if parallel mode was selected
    if selection == all_items.len() - 1 {
        return Ok(Some(WizardSelection::Parallel));
    }

    let selected_spec = ready_specs[selection].clone();

    // Show prompt selection
    let available_prompts = super::list_available_prompts(prompts_dir)?;

    if available_prompts.is_empty() {
        anyhow::bail!("No prompts found in {}", prompts_dir.display());
    }

    let prompt_selection = Select::new()
        .with_prompt("? Select prompt")
        .items(&available_prompts)
        .default(0)
        .interact()?;

    let selected_prompt = available_prompts[prompt_selection].clone();

    // Show branch confirmation
    let create_branch = dialoguer::Confirm::new()
        .with_prompt("Create feature branch")
        .default(false)
        .interact()?;

    Ok(Some(WizardSelection::SingleSpec {
        spec_id: selected_spec.id,
        prompt: selected_prompt,
        create_branch,
    }))
}

/// Auto-select a prompt based on spec type if the prompt file exists.
/// Returns None if no auto-selected prompt is appropriate or available.
#[allow(dead_code)]
pub fn auto_select_prompt_for_type(spec: &Spec, prompts_dir: &Path) -> Option<String> {
    let auto_prompt = match spec.frontmatter.r#type {
        SpecType::Documentation => Some("documentation"),
        _ => None,
    };

    // Check if the auto-selected prompt actually exists
    if let Some(prompt_name) = auto_prompt {
        let prompt_path = prompts_dir.join(format!("{}.md", prompt_name));
        if prompt_path.exists() {
            return Some(prompt_name.to_string());
        }
    }

    None
}
