use std::fs;
use std::path::PathBuf;

pub fn load_fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{}.md", name));

    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{}' from {:?}: {}", name, path, e))
}
