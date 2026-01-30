//! Site generation commands
//!
//! Commands for generating static documentation sites from specs.

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::site::{theme, SiteGenerator};
use chant::spec::Spec;

/// Path to the theme directory
const THEME_DIR: &str = ".chant/site/theme";

/// Initialize the theme directory with default templates
pub fn cmd_site_init(force: bool) -> Result<()> {
    let _specs_dir = crate::cmd::ensure_initialized()?;

    let theme_dir = PathBuf::from(THEME_DIR);

    if theme_dir.exists() && !force {
        println!(
            "{} Theme directory already exists at {}",
            "Note:".cyan(),
            theme_dir.display()
        );
        println!("Use {} to overwrite existing files", "--force".cyan());

        // List existing files
        let files = theme::list_theme_files(&theme_dir)?;
        if !files.is_empty() {
            println!("\n{}", "Existing theme files:".bold());
            for file in files {
                println!("  {}", file);
            }
        }
        return Ok(());
    }

    // Create parent directory
    if let Some(parent) = theme_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    let result = theme::init_theme(&theme_dir, force)?;

    if result.has_changes() {
        println!(
            "{} Theme initialized at {}",
            "✓".green(),
            theme_dir.display()
        );
        println!("\n{}", "Created files:".bold());
        for file in &result.created {
            let info = theme::get_theme_files()
                .iter()
                .find(|f| f.name == file)
                .map(|f| f.description)
                .unwrap_or("");
            println!("  {} - {}", file.cyan(), info.dimmed());
        }

        if !result.skipped.is_empty() {
            println!("\n{}", "Skipped (already exist):".yellow());
            for file in &result.skipped {
                println!("  {}", file);
            }
        }

        println!("\n{}", "Next steps:".bold());
        println!("  1. Edit templates in {}", theme_dir.display());
        println!(
            "  2. Run {} to generate the site",
            "chant site build".cyan()
        );
        println!("  3. Run {} to preview locally", "chant site serve".cyan());

        // Add template variables documentation hint
        println!(
            "\n{} See {} for template variable documentation",
            "Tip:".cyan(),
            "chant site init --help".dimmed()
        );
    } else {
        println!(
            "{} No files created (all exist). Use {} to overwrite.",
            "Note:".yellow(),
            "--force".cyan()
        );
    }

    Ok(())
}

/// Build the static site
pub fn cmd_site_build(output: Option<&str>) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load configuration
    let config = Config::load()?;

    // Determine output directory
    let output_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&config.site.output_dir));

    println!("{} Building site to {}", "→".cyan(), output_dir.display());

    // Load all specs
    let specs = load_specs(&specs_dir)?;
    println!("  Found {} specs", specs.len());

    // Check for custom theme
    let theme_dir = PathBuf::from(THEME_DIR);
    let theme_path = if theme_dir.exists() {
        println!("  Using custom theme from {}", theme_dir.display());
        Some(theme_dir.as_path())
    } else {
        println!("  Using embedded default theme");
        None
    };

    // Build the site
    let generator = SiteGenerator::new(config.site.clone(), specs, theme_path)?;
    let result = generator.build(&output_dir)?;

    println!("\n{} Site built successfully", "✓".green());
    println!("  {} specs included", result.specs_included);
    println!("  {} files written", result.files_written);
    println!("  Output: {}", output_dir.display());

    println!("\n{}", "Next steps:".bold());
    println!(
        "  Preview locally: {}",
        "chant site serve --port 3000".cyan()
    );
    println!("  Deploy: Copy {} to your web server", output_dir.display());

    Ok(())
}

/// Start a local HTTP server to preview the site
pub fn cmd_site_serve(port: u16, output: Option<&str>) -> Result<()> {
    let _specs_dir = crate::cmd::ensure_initialized()?;

    // Load configuration for default output dir
    let config = Config::load()?;

    let output_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&config.site.output_dir));

    if !output_dir.exists() {
        anyhow::bail!(
            "Site not built yet. Run {} first.",
            "chant site build".cyan()
        );
    }

    // Check if index.html exists
    if !output_dir.join("index.html").exists() {
        anyhow::bail!(
            "No index.html found in {}. Run {} first.",
            output_dir.display(),
            "chant site build".cyan()
        );
    }

    // Start simple HTTP server
    let addr = format!("127.0.0.1:{}", port);
    let listener =
        TcpListener::bind(&addr).with_context(|| format!("Failed to bind to {}", addr))?;

    println!(
        "{} Serving {} at {}",
        "→".cyan(),
        output_dir.display(),
        format!("http://{}", addr).green()
    );
    println!("Press {} to stop", "Ctrl+C".yellow());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Read request
                let mut buffer = [0; 4096];
                if stream.read(&mut buffer).is_err() {
                    continue;
                }

                let request = String::from_utf8_lossy(&buffer);
                let path = parse_request_path(&request);

                // Serve file
                let file_path = resolve_file_path(&output_dir, &path);
                let (status, content_type, body) = read_file(&file_path);

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                    status,
                    content_type,
                    body.len()
                );

                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(&body);

                // Log request
                let status_code = status.split(' ').next().unwrap_or("???");
                let status_color = if status_code == "200" {
                    status_code.green()
                } else {
                    status_code.yellow()
                };
                println!(
                    "  {} {} {}",
                    status_color,
                    path,
                    file_path.display().to_string().dimmed()
                );
            }
            Err(e) => {
                eprintln!("{} Connection error: {}", "Error:".red(), e);
            }
        }
    }

    Ok(())
}

/// Load all specs from the specs directory
fn load_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    let mut specs = Vec::new();

    for entry in fs::read_dir(specs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            match Spec::load(&path) {
                Ok(spec) => specs.push(spec),
                Err(e) => {
                    eprintln!(
                        "{} Failed to load {}: {}",
                        "Warning:".yellow(),
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    // Sort by ID (most recent first)
    specs.sort_by(|a, b| b.id.cmp(&a.id));

    Ok(specs)
}

/// Parse the request path from an HTTP request
fn parse_request_path(request: &str) -> String {
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() >= 2 {
        let path = parts[1];
        // Remove query string
        let path = path.split('?').next().unwrap_or(path);
        // URL decode
        urlencoding_decode(path)
    } else {
        "/".to_string()
    }
}

/// Simple URL decoding
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Resolve the file path for a request
fn resolve_file_path(root: &Path, request_path: &str) -> PathBuf {
    let path = request_path.trim_start_matches('/');

    let file_path = if path.is_empty() || path.ends_with('/') {
        root.join(path).join("index.html")
    } else {
        let full_path = root.join(path);
        if full_path.is_dir() {
            full_path.join("index.html")
        } else {
            full_path
        }
    };

    // Security: ensure path is within root
    if let Ok(canonical) = file_path.canonicalize() {
        if let Ok(root_canonical) = root.canonicalize() {
            if canonical.starts_with(&root_canonical) {
                return canonical;
            }
        }
    }

    file_path
}

/// Read a file and return status, content type, and body
fn read_file(path: &Path) -> (&'static str, &'static str, Vec<u8>) {
    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    };

    match fs::read(path) {
        Ok(body) => ("200 OK", content_type, body),
        Err(_) => (
            "404 Not Found",
            "text/html; charset=utf-8",
            b"<h1>404 Not Found</h1>".to_vec(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_path() {
        assert_eq!(parse_request_path("GET / HTTP/1.1"), "/");
        assert_eq!(
            parse_request_path("GET /index.html HTTP/1.1"),
            "/index.html"
        );
        assert_eq!(
            parse_request_path("GET /specs/test.html?v=1 HTTP/1.1"),
            "/specs/test.html"
        );
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("test+value"), "test value");
        assert_eq!(urlencoding_decode("normal"), "normal");
    }
}
