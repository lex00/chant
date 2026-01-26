//! Spec ID generation with date-based sequencing.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/ids.md, reference/schema.md
//! - ignore: false

use anyhow::Result;
use chrono::Local;
use rand::Rng;
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
}
