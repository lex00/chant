fn main() {
    // Get git commit hash - prefer env var (set by Docker build) over git command
    let git_sha = std::env::var("GIT_SHA").unwrap_or_else(|_| {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    });

    println!("cargo:rustc-env=GIT_SHA={}", git_sha);

    // Get build date - prefer env var (set by Docker build) over date command
    let build_date = std::env::var("BUILD_DATE").unwrap_or_else(|_| {
        std::process::Command::new("date")
            .arg("+%Y-%m-%d")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    });

    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
}
