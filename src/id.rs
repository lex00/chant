//! Spec ID generation with date-based sequencing.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/ids.md, reference/schema.md
//! - ignore: false

use anyhow::{anyhow, Result};
use chrono::Local;
use rand::Rng;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

const BASE36_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Generate a new spec ID in the format: YYYY-MM-DD-SSS-XXX
/// where SSS is a base36 sequence and XXX is a random base36 suffix.
pub fn generate_id(specs_dir: &Path) -> Result<String> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let seq = next_sequence_for_date(specs_dir, &date)?;
    let rand = random_base36(3);

    Ok(format!("{}-{}-{}", date, format_base36(seq, 3), rand))
}

/// Get the next sequence number for a given date.
fn next_sequence_for_date(specs_dir: &Path, date: &str) -> Result<u32> {
    let mut max_seq = 0u32;

    if specs_dir.exists() {
        for entry in std::fs::read_dir(specs_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let name = filename.to_string_lossy();

            // Match pattern: YYYY-MM-DD-SSS-XXX.md or YYYY-MM-DD-SSS-XXX.N.md (group member)
            if name.starts_with(date) && name.ends_with(".md") {
                // Extract the sequence part (after the date, before the random suffix)
                let parts: Vec<&str> = name.trim_end_matches(".md").split('-').collect();
                if parts.len() >= 5 {
                    // parts: [YYYY, MM, DD, SSS, XXX] or [YYYY, MM, DD, SSS, XXX.N]
                    if let Some(seq) = parse_base36(parts[3]) {
                        max_seq = max_seq.max(seq);
                    }
                }
            }
        }
    }

    Ok(max_seq + 1)
}

/// Format a number as base36 with zero-padding.
pub fn format_base36(n: u32, width: usize) -> String {
    if n == 0 {
        return "0".repeat(width);
    }

    let mut result = Vec::new();
    let mut num = n;

    while num > 0 {
        let digit = (num % 36) as usize;
        result.push(BASE36_CHARS[digit] as char);
        num /= 36;
    }

    result.reverse();
    let s: String = result.into_iter().collect();

    if s.len() < width {
        format!("{:0>width$}", s, width = width)
    } else {
        s
    }
}

/// Parse a base36 string to a number.
fn parse_base36(s: &str) -> Option<u32> {
    let mut result = 0u32;

    for c in s.chars() {
        result *= 36;
        if let Some(pos) = BASE36_CHARS.iter().position(|&b| b as char == c) {
            result += pos as u32;
        } else {
            return None;
        }
    }

    Some(result)
}

/// Generate a random base36 string of the given length.
fn random_base36(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| BASE36_CHARS[rng.gen_range(0..36)] as char)
        .collect()
}

/// Represents a parsed spec ID with optional repo prefix.
///
/// Spec IDs can have three formats:
/// - `2026-01-27-001-abc` (local spec, no repo prefix)
/// - `project-2026-01-27-001-abc` (local spec with project prefix)
/// - `backend:2026-01-27-001-abc` (cross-repo spec without project)
/// - `backend:auth-2026-01-27-001-abc` (cross-repo spec with project)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecId {
    pub repo: Option<String>,
    pub project: Option<String>,
    pub base_id: String,
    pub member: Option<u32>,
}

impl SpecId {
    /// Parse a spec ID string into a SpecId struct.
    ///
    /// Supports formats:
    /// - `2026-01-27-001-abc` - local spec
    /// - `project-2026-01-27-001-abc` - local spec with project
    /// - `backend:2026-01-27-001-abc` - cross-repo spec
    /// - `backend:project-2026-01-27-001-abc` - cross-repo with project
    /// - Any of above with `.N` suffix for group members
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Repo name contains invalid characters (not alphanumeric, hyphen, or underscore)
    /// - Repo name is empty
    /// - Base ID format is invalid
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(anyhow!("Spec ID cannot be empty"));
        }

        // Check for repo prefix (first `:`)
        let (repo, remainder) = if let Some(colon_pos) = input.find(':') {
            let repo_name = &input[..colon_pos];
            if repo_name.is_empty() {
                return Err(anyhow!("Repo name cannot be empty before ':'"));
            }
            if !is_valid_repo_name(repo_name) {
                return Err(anyhow!("Invalid repo name '{}': must contain only alphanumeric characters, hyphens, and underscores", repo_name));
            }
            (Some(repo_name.to_string()), &input[colon_pos + 1..])
        } else {
            (None, input)
        };

        // Parse the remainder as: [project-]base_id[.member]
        let (base_id, member) = Self::parse_base_id(remainder)?;

        // Check if base_id has a project prefix
        let (project, base_id) = Self::extract_project(&base_id)?;

        Ok(SpecId {
            repo,
            project,
            base_id,
            member,
        })
    }

    /// Parse the base ID part, handling member suffixes.
    fn parse_base_id(input: &str) -> Result<(String, Option<u32>)> {
        if input.is_empty() {
            return Err(anyhow!("Base ID cannot be empty"));
        }

        // Check for member suffix (.N)
        if let Some(dot_pos) = input.rfind('.') {
            let (base, suffix) = input.split_at(dot_pos);
            // Check if suffix is numeric
            if suffix.len() > 1 {
                let num_str = &suffix[1..];
                // Check if the first part after dot is numeric
                if let Some(first_char) = num_str.chars().next() {
                    if first_char.is_ascii_digit() {
                        // Try to parse as member number
                        let member_part: String =
                            num_str.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if let Ok(member_num) = member_part.parse::<u32>() {
                            return Ok((base.to_string(), Some(member_num)));
                        }
                    }
                }
            }
        }

        Ok((input.to_string(), None))
    }

    /// Extract project prefix from base_id if present.
    /// Project prefix format: `project-rest` where project is alphanumeric with hyphens/underscores.
    /// Looks for a 4-digit year (YYYY) in the string to detect the start of the date part.
    fn extract_project(base_id: &str) -> Result<(Option<String>, String)> {
        let parts: Vec<&str> = base_id.split('-').collect();

        // Need at least 5 parts for any valid spec ID: [YYYY, MM, DD, SSS, XXX]
        // If less than 5 parts, no project prefix possible
        if parts.len() < 5 {
            return Ok((None, base_id.to_string()));
        }

        // Check if parts[0] looks like a year (YYYY)
        if parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit()) {
            // No project prefix, starts with year directly
            return Ok((None, base_id.to_string()));
        }

        // Look for a 4-digit year anywhere in the parts after position 0
        for i in 1..parts.len() {
            if parts[i].len() == 4 && parts[i].chars().all(|c| c.is_ascii_digit()) {
                // Found a year at position i, everything before is the project
                let project = parts[0..i].join("-");
                let rest = parts[i..].join("-");
                return Ok((Some(project), rest));
            }
        }

        // No year found, treat as no project
        Ok((None, base_id.to_string()))
    }
}

impl Display for SpecId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(repo) = &self.repo {
            write!(f, "{}:", repo)?;
        }
        if let Some(project) = &self.project {
            write!(f, "{}-", project)?;
        }
        write!(f, "{}", self.base_id)?;
        if let Some(member) = self.member {
            write!(f, ".{}", member)?;
        }
        Ok(())
    }
}

/// Check if a string is a valid repo name.
/// Valid names contain only alphanumeric characters, hyphens, and underscores.
fn is_valid_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_base36() {
        assert_eq!(format_base36(0, 3), "000");
        assert_eq!(format_base36(1, 3), "001");
        assert_eq!(format_base36(10, 3), "00a");
        assert_eq!(format_base36(35, 3), "00z");
        assert_eq!(format_base36(36, 3), "010");
        assert_eq!(format_base36(999, 3), "0rr");
        assert_eq!(format_base36(1000, 3), "0rs");
    }

    #[test]
    fn test_parse_base36() {
        assert_eq!(parse_base36("000"), Some(0));
        assert_eq!(parse_base36("001"), Some(1));
        assert_eq!(parse_base36("00a"), Some(10));
        assert_eq!(parse_base36("00z"), Some(35));
        assert_eq!(parse_base36("010"), Some(36));
    }

    #[test]
    fn test_random_base36_length() {
        let r = random_base36(3);
        assert_eq!(r.len(), 3);
        assert!(r.chars().all(|c| BASE36_CHARS.contains(&(c as u8))));
    }

    // SpecId tests

    #[test]
    fn test_parse_local_id_without_project() {
        let spec = SpecId::parse("2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, None);
        assert_eq!(spec.project, None);
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, None);
    }

    #[test]
    fn test_parse_local_id_with_project() {
        let spec = SpecId::parse("auth-2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, None);
        assert_eq!(spec.project, Some("auth".to_string()));
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, None);
    }

    #[test]
    fn test_parse_repo_id_without_project() {
        let spec = SpecId::parse("backend:2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, Some("backend".to_string()));
        assert_eq!(spec.project, None);
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, None);
    }

    #[test]
    fn test_parse_repo_id_with_project() {
        let spec = SpecId::parse("backend:auth-2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, Some("backend".to_string()));
        assert_eq!(spec.project, Some("auth".to_string()));
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, None);
    }

    #[test]
    fn test_parse_local_id_with_member() {
        let spec = SpecId::parse("2026-01-27-001-abc.1").unwrap();
        assert_eq!(spec.repo, None);
        assert_eq!(spec.project, None);
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, Some(1));
    }

    #[test]
    fn test_parse_local_id_with_project_and_member() {
        let spec = SpecId::parse("auth-2026-01-27-001-abc.3").unwrap();
        assert_eq!(spec.repo, None);
        assert_eq!(spec.project, Some("auth".to_string()));
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, Some(3));
    }

    #[test]
    fn test_parse_repo_id_with_member() {
        let spec = SpecId::parse("backend:2026-01-27-001-abc.2").unwrap();
        assert_eq!(spec.repo, Some("backend".to_string()));
        assert_eq!(spec.project, None);
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, Some(2));
    }

    #[test]
    fn test_parse_repo_id_with_project_and_member() {
        let spec = SpecId::parse("backend:auth-2026-01-27-001-abc.5").unwrap();
        assert_eq!(spec.repo, Some("backend".to_string()));
        assert_eq!(spec.project, Some("auth".to_string()));
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
        assert_eq!(spec.member, Some(5));
    }

    #[test]
    fn test_parse_repo_with_hyphen() {
        let spec = SpecId::parse("my-repo:2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, Some("my-repo".to_string()));
        assert_eq!(spec.project, None);
        assert_eq!(spec.base_id, "2026-01-27-001-abc");
    }

    #[test]
    fn test_parse_repo_with_underscore() {
        let spec = SpecId::parse("my_repo:2026-01-27-001-abc").unwrap();
        assert_eq!(spec.repo, Some("my_repo".to_string()));
    }

    #[test]
    fn test_parse_project_with_hyphen() {
        let spec = SpecId::parse("auth-service-2026-01-27-001-abc").unwrap();
        assert_eq!(spec.project, Some("auth-service".to_string()));
    }

    #[test]
    fn test_invalid_repo_name_empty_before_colon() {
        let result = SpecId::parse(":2026-01-27-001-abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_invalid_repo_name_with_special_chars() {
        let result = SpecId::parse("back@end:2026-01-27-001-abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_repo_name_with_dot() {
        let result = SpecId::parse("backend.com:2026-01-27-001-abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_spec_id() {
        let result = SpecId::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_display_local_id() {
        let spec = SpecId {
            repo: None,
            project: None,
            base_id: "2026-01-27-001-abc".to_string(),
            member: None,
        };
        assert_eq!(spec.to_string(), "2026-01-27-001-abc");
    }

    #[test]
    fn test_display_local_id_with_project() {
        let spec = SpecId {
            repo: None,
            project: Some("auth".to_string()),
            base_id: "2026-01-27-001-abc".to_string(),
            member: None,
        };
        assert_eq!(spec.to_string(), "auth-2026-01-27-001-abc");
    }

    #[test]
    fn test_display_repo_id() {
        let spec = SpecId {
            repo: Some("backend".to_string()),
            project: None,
            base_id: "2026-01-27-001-abc".to_string(),
            member: None,
        };
        assert_eq!(spec.to_string(), "backend:2026-01-27-001-abc");
    }

    #[test]
    fn test_display_repo_id_with_project() {
        let spec = SpecId {
            repo: Some("backend".to_string()),
            project: Some("auth".to_string()),
            base_id: "2026-01-27-001-abc".to_string(),
            member: None,
        };
        assert_eq!(spec.to_string(), "backend:auth-2026-01-27-001-abc");
    }

    #[test]
    fn test_display_with_member() {
        let spec = SpecId {
            repo: Some("backend".to_string()),
            project: Some("auth".to_string()),
            base_id: "2026-01-27-001-abc".to_string(),
            member: Some(3),
        };
        assert_eq!(spec.to_string(), "backend:auth-2026-01-27-001-abc.3");
    }

    #[test]
    fn test_parse_and_display_roundtrip() {
        let inputs = vec![
            "2026-01-27-001-abc",
            "auth-2026-01-27-001-abc",
            "backend:2026-01-27-001-abc",
            "backend:auth-2026-01-27-001-abc",
            "2026-01-27-001-abc.1",
            "auth-2026-01-27-001-abc.2",
            "backend:2026-01-27-001-abc.3",
            "backend:auth-2026-01-27-001-abc.4",
            "my-repo:my-proj-2026-01-27-001-abc.5",
        ];

        for input in inputs {
            let spec = SpecId::parse(input).unwrap();
            assert_eq!(spec.to_string(), input);
        }
    }

    #[test]
    fn test_parse_member_with_large_number() {
        let spec = SpecId::parse("2026-01-27-001-abc.999").unwrap();
        assert_eq!(spec.member, Some(999));
    }
}
