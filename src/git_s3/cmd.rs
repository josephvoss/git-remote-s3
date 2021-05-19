/// Mod to run git commands live in repository

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Check if new hash is a fast-forward of the old hash
pub fn is_ancestor(git_dir: &PathBuf, old_hash: &str, new_hash: &str) -> Result<bool> {
    let output = Command::new("git").arg("merge-base").arg("--is-ancestor")
        .arg(old_hash).arg(new_hash).env("GIT_DIR", git_dir)
        .output()
        .with_context(|| format!("Failed to check is_ancestor for {} to {}", old_hash, new_hash))?;

    Ok(output.status.success())
}
