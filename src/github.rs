use anyhow::{Context, Result};
use std::process::Command;

pub struct GitHub {}

impl GitHub {
    pub async fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn get_pr_branch(&self, pr_number: u64) -> Result<String> {
        let output = Command::new("gh")
            .args(["pr", "view", &pr_number.to_string(), "--json", "headRefName", "--jq", ".headRefName"])
            .output()
            .context("Failed to execute gh command")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get PR branch: {}", String::from_utf8_lossy(&output.stderr));
        }

        let branch = String::from_utf8(output.stdout)
            .context("Failed to parse gh command output")?
            .trim()
            .to_string();

        Ok(branch)
    }

    pub async fn is_pr_merged(&self, pr_number: u64) -> Result<bool> {
        let output = Command::new("gh")
            .args(["pr", "view", &pr_number.to_string(), "--json", "state", "--jq", ".state"])
            .output()
            .context("Failed to execute gh command")?;

        if !output.status.success() {
            return Ok(false);
        }

        let state = String::from_utf8(output.stdout)
            .context("Failed to parse gh command output")?
            .trim()
            .to_string();

        Ok(state == "MERGED")
    }

    pub fn extract_pr_number_from_worktree(&self, worktree: &str) -> Option<u64> {
        let path = std::path::Path::new(worktree);
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse::<u64>().ok())
    }
} 