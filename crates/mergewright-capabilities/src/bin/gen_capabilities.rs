//! Regenerates `CAPABILITIES.md` and `docs/generated/capabilities.html`
//! from `docs/capabilities/*.md`.
//!
//! Run from anywhere inside the workspace:
//!
//! ```text
//! cargo run -p mergewright-capabilities --bin gen-capabilities
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let repo_root = match repo_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("gen-capabilities: {message}");
            return ExitCode::FAILURE;
        }
    };

    let capabilities = match mergewright_capabilities::load(&repo_root) {
        Ok(caps) => caps,
        Err(error) => {
            eprintln!("gen-capabilities: {error}");
            return ExitCode::FAILURE;
        }
    };

    let findings = mergewright_capabilities::check(&repo_root, &capabilities);
    if !findings.is_empty() {
        eprintln!(
            "gen-capabilities: {} citation(s) do not check out (regenerating anyway; \
             `cargo nextest run -p mergewright-capabilities` will fail until these are fixed):",
            findings.len()
        );
        for finding in &findings {
            eprintln!("  {finding}");
        }
    }

    let markdown = mergewright_capabilities::render_markdown(&capabilities);
    let markdown_path = repo_root.join("CAPABILITIES.md");
    if let Err(error) = std::fs::write(&markdown_path, markdown) {
        eprintln!(
            "gen-capabilities: could not write {}: {error}",
            markdown_path.display()
        );
        return ExitCode::FAILURE;
    }
    println!("wrote {}", markdown_path.display());

    let html = mergewright_capabilities::render_html(&capabilities);
    let html_path = repo_root.join("docs/generated/capabilities.html");
    if let Some(parent) = html_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "gen-capabilities: could not create {}: {error}",
            parent.display()
        );
        return ExitCode::FAILURE;
    }
    if let Err(error) = std::fs::write(&html_path, html) {
        eprintln!(
            "gen-capabilities: could not write {}: {error}",
            html_path.display()
        );
        return ExitCode::FAILURE;
    }
    println!("wrote {}", html_path.display());

    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Walks up from this binary's crate directory to the workspace root (the
/// directory holding the top-level `Cargo.toml` with a `[workspace]`
/// table), so the tool works regardless of the caller's cwd.
fn repo_root() -> Result<PathBuf, String> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if is_workspace_root(&dir) {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find the workspace root above this crate".to_owned());
        }
    }
}

fn is_workspace_root(dir: &Path) -> bool {
    let manifest = dir.join("Cargo.toml");
    match std::fs::read_to_string(&manifest) {
        Ok(content) => content.contains("[workspace]"),
        Err(_) => false,
    }
}
