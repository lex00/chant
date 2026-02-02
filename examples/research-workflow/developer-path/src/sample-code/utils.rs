// TODO: Move these to a proper utility crate

pub fn validate_email(email: &str) -> bool {
    // FIXME: This is a very naive email validator
    email.contains('@') && email.contains('.')
}

pub fn sanitize_input(input: &str) -> String {
    // TODO: Add proper input sanitization
    input.replace('<', "").replace('>', "")
}

// WARNING: This function has high cyclomatic complexity
pub fn process_user_input(input: &str, mode: &str, options: Vec<&str>) -> Result<String, String> {
    if input.is_empty() {
        return Err("Empty input".to_string());
    }

    let sanitized = sanitize_input(input);

    if mode == "strict" {
        if sanitized.len() > 100 {
            return Err("Input too long".to_string());
        }
        if !validate_email(&sanitized) && options.contains(&"require_email") {
            return Err("Invalid email".to_string());
        }
        if sanitized.contains("admin") && !options.contains(&"allow_admin") {
            return Err("Admin keyword not allowed".to_string());
        }
    } else if mode == "lenient" {
        if sanitized.len() > 500 {
            return Err("Input too long".to_string());
        }
    } else if mode == "custom" {
        // TODO: Implement custom validation rules
        if options.is_empty() {
            return Err("Custom mode requires options".to_string());
        }
        for option in options {
            if option == "no_special_chars" && sanitized.chars().any(|c| !c.is_alphanumeric()) {
                return Err("Special characters not allowed".to_string());
            }
        }
    } else {
        return Err("Unknown mode".to_string());
    }

    Ok(sanitized)
}

// FIXME: No error handling
pub fn parse_config(config_str: &str) -> HashMap<String, String> {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    for line in config_str.lines() {
        // TODO: Handle malformed lines gracefully
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    map
}
