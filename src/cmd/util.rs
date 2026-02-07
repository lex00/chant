//! Utility commands (version, man page generation, completion).

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;
use std::path::PathBuf;

use crate::cli::Cli;

/// Show version information
pub fn cmd_version(verbose: bool) -> Result<()> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("chant {}", VERSION);

    if verbose {
        const GIT_SHA: &str = env!("GIT_SHA");
        const BUILD_DATE: &str = env!("BUILD_DATE");
        println!("commit: {}", GIT_SHA);
        println!("built: {}", BUILD_DATE);
    }

    Ok(())
}

/// Generate man page
pub fn cmd_man(out_dir: Option<&PathBuf>) -> Result<()> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer)?;

    let output_dir = out_dir
        .map(|p| p.to_owned())
        .unwrap_or_else(|| PathBuf::from("."));

    std::fs::create_dir_all(&output_dir)?;
    let man_path = output_dir.join("chant.1");
    std::fs::write(&man_path, buffer)?;

    println!("Man page written to: {}", man_path.display());
    Ok(())
}

/// Generate shell completion script
pub fn cmd_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "chant", &mut io::stdout());
    Ok(())
}
